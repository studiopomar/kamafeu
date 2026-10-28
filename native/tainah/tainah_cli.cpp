#include "tainah.h"
#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
uint16_t u16(const unsigned char *p) { return uint16_t(p[0]) | uint16_t(p[1]) << 8; }
uint32_t u32(const unsigned char *p) { return uint32_t(p[0]) | uint32_t(p[1]) << 8 | uint32_t(p[2]) << 16 | uint32_t(p[3]) << 24; }
bool read_wav(const std::string &path, std::vector<float> &mono, uint32_t &rate) {
    std::ifstream f(path, std::ios::binary);
    if (!f) return false;
    std::vector<unsigned char> bytes((std::istreambuf_iterator<char>(f)), {});
    if (bytes.size() < 44 || std::memcmp(bytes.data(), "RIFF", 4) || std::memcmp(bytes.data()+8, "WAVE", 4)) return false;
    uint16_t format=0, channels=0, bits=0; uint32_t data_size=0; size_t data=0;
    for (size_t p=12; p+8<=bytes.size();) {
        uint32_t n=u32(bytes.data()+p+4); size_t body=p+8;
        if (body+n>bytes.size()) return false;
        if (!std::memcmp(bytes.data()+p,"fmt ",4) && n>=16) { format=u16(bytes.data()+body); channels=u16(bytes.data()+body+2); rate=u32(bytes.data()+body+4); bits=u16(bytes.data()+body+14); }
        if (!std::memcmp(bytes.data()+p,"data",4)) { data=body; data_size=n; }
        p=body+n+(n&1);
    }
    if (!data || !channels || !rate || (format!=1 && !(format==3 && bits==32)) ||
        (bits!=8 && bits!=16 && bits!=24 && bits!=32)) return false;
    size_t stride=channels*(bits/8); if (!stride || data_size%stride) return false;
    auto sample=[&](const unsigned char *p)->float {
        if (format==3 && bits==32) { float v; std::memcpy(&v,p,4); return v; }
        if (bits==8) return (int(p[0])-128)/128.f;
        if (bits==16) return int16_t(u16(p))/32768.f;
        if (bits==24) { int32_t v=int32_t(p[0])|int32_t(p[1])<<8|int32_t(p[2])<<16; if(v&0x800000)v|=~0xffffff; return v/8388608.f; }
        int32_t v=static_cast<int32_t>(u32(p)); return v/2147483648.f;
    };
    mono.resize(data_size/stride);
    for(size_t i=0;i<mono.size();++i) { double sum=0; for(uint16_t c=0;c<channels;++c) sum+=sample(bytes.data()+data+i*stride+c*(bits/8)); mono[i]=float(sum/channels); }
    return true;
}
bool write_wav(const std::string &path,const lt_pcm &pcm,double gain) {
    if(pcm.length>UINT32_MAX/2-44) return false;
    std::ofstream f(path,std::ios::binary); if(!f)return false;
    uint32_t data=uint32_t(pcm.length*2), riff=data+36, rate=pcm.sample_rate;
    auto w16=[&](uint16_t x){f.put(char(x));f.put(char(x>>8));};
    auto w32=[&](uint32_t x){w16(uint16_t(x));w16(uint16_t(x>>16));};
    f.write("RIFF",4);w32(riff);f.write("WAVEfmt ",8);w32(16);w16(1);w16(1);w32(rate);w32(rate*2);w16(2);w16(16);f.write("data",4);w32(data);
    for(uint64_t i=0;i<pcm.length;++i){double v=std::clamp(double(pcm.samples[i])*gain,-1.0,1.0);int16_t s=int16_t(std::lrint(v*32767));w16(uint16_t(s));}
    return bool(f);
}
int note_number(std::string s) {
    static const char *names[]={"C","C#","D","D#","E","F","F#","G","G#","A","A#","B"};
    if(s.size()<2)return -1; int octave; try{octave=std::stoi(s.substr(s.find_last_not_of("0123456789")+1));}catch(...){return -1;}
    std::string key=s.substr(0,s.find_last_not_of("0123456789")+1);
    for(int i=0;i<12;++i)if(key==names[i])return (octave+1)*12+i; return -1;
}
std::vector<double> decode_pitch(const std::string &s) {
    const std::string alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    std::vector<int> values;
    for(size_t i=0;i+1<s.size();) {
        if(s[i]=='#') { size_t e=s.find('#',i+1); if(e==std::string::npos)break; int count=0; try{count=std::stoi(s.substr(i+1,e-i-1));}catch(...){break;} if(!values.empty())for(int n=0;n<std::clamp(count,0,100000);++n)values.push_back(values.back()); i=e+1; continue; }
        auto a=alphabet.find(s[i]),b=alphabet.find(s[i+1]); if(a==std::string::npos||b==std::string::npos)break;
        int v=int((a<<6)|b); if(v&0x800)v-=4096; values.push_back(v); i+=2;
    }
    std::vector<double> result; result.reserve(values.size()); for(int v:values)result.push_back(v); return result;
}
double flag_value(const std::string &flags,char key,double fallback=0) {
    auto p=flags.find(key); if(p==std::string::npos)return fallback; size_t end=p+1; if(end<flags.size()&&(flags[end]=='-'||flags[end]=='+'))++end; while(end<flags.size()&&std::isdigit(static_cast<unsigned char>(flags[end])))++end;
    try{return std::stod(flags.substr(p+1,end-p-1));}catch(...){return fallback;}
}
}

int main(int argc,char **argv) {
    if(argc<13){std::cerr<<"Uso: Tainah entrada.wav saida.wav nota velocidade flags offset duracao consoante cutoff volume modulacao !tempo [curva]\n";return 2;}
    try {
        std::vector<float> input; uint32_t rate=0;
        if(!read_wav(argv[1],input,rate)){std::cerr<<"Não foi possível ler o WAV de entrada (PCM ou IEEE float).\n";return 3;}
        int note=note_number(argv[3]); if(note<0){std::cerr<<"Nota inválida: "<<argv[3]<<"\n";return 2;}
        double velocity=std::stod(argv[4]), offset=std::stod(argv[6]), duration=std::stod(argv[7]);
        double consonant=std::stod(argv[8]), cutoff=std::stod(argv[9]), volume=std::stod(argv[10]), modulation=std::stod(argv[11]);
        std::string tempo_arg = argv[12];
        if (!tempo_arg.empty() && tempo_arg.front() == '!') tempo_arg.erase(0, 1);
        size_t tempo_end = 0;
        const double tempo = std::stod(tempo_arg, &tempo_end);
        if (tempo_end != tempo_arg.size() || !std::isfinite(tempo) || tempo <= 0)
            throw std::invalid_argument("andamento inválido");
        if (!std::isfinite(velocity) || !std::isfinite(modulation) || !std::isfinite(volume))
            throw std::invalid_argument("volume, velocidade ou modulação inválida");
        lt_render_options o{}; o.struct_size=sizeof(o);o.sample_rate=rate;o.input=input.data();o.input_length=input.size();o.duration_ms=duration;o.offset_ms=offset;o.consonant_ms=consonant;o.cutoff_ms=cutoff;
        o.has_target_consonant=1;
        o.target_consonant_ms=std::max(0.0,consonant)*std::pow(2.0,1.0-std::clamp(velocity,-100.0,200.0)/100.0);
        o.pitch_hz=440*std::pow(2.0,(note-69)/12.0);o.gender=flag_value(argv[5],'g');o.pitch_cents=flag_value(argv[5],'t')*10.0;o.breathiness=flag_value(argv[5],'B');
        o.formant_shift=std::clamp(flag_value(argv[5],'P'),-100.0,100.0);
        o.consonant_adjustment=std::clamp(flag_value(argv[5],'C'),-100.0,100.0);
        const double high=std::clamp(flag_value(argv[5],'H'),0.0,99.0), nyquist=rate/2.0;
        o.high_frequency_cutoff=high<=0.0||nyquist<=1000.0?0.0:nyquist*std::pow(1000.0/nyquist,high/99.0);
        o.frame_period_ms=5;o.f0_floor=55;o.f0_ceiling=1760;o.f0_method=1;
        if(argc>13){auto curve=decode_pitch(argv[13]); static std::vector<lt_pitch_point> points; for(size_t i=0;i<curve.size();++i)points.push_back({double(i)*60000.0*5.0/(tempo*480.0),curve[i],1}); o.pitch_points=points.data();o.pitch_point_count=uint32_t(points.size());}
        lt_pcm pcm{};pcm.struct_size=sizeof(pcm);lt_error error{};error.struct_size=sizeof(error);
        int status=lt_render(&o,&pcm,&error);if(status){std::cerr<<"Falha ao renderizar: "<<error.message<<"\n";return status;}
        const double gain=std::clamp(volume/100.0,0.0,4.0);
        bool ok=write_wav(argv[2],pcm,gain);lt_pcm_free(&pcm);if(!ok){std::cerr<<"Não foi possível gravar o WAV de saída.\n";return 4;}
        return 0;
    } catch(const std::exception &e){std::cerr<<"Parâmetro inválido: "<<e.what()<<"\n";return 2;}
}
