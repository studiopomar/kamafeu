#include <ftxui/component/component.hpp>
#include <ftxui/component/event.hpp>
#include <ftxui/component/screen_interactive.hpp>
#include <ftxui/dom/elements.hpp>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdlib>
#include <iostream>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <unistd.h>
#endif

using namespace ftxui;

namespace {

struct Record {
  std::string level;
  std::string scope;
  std::string message;
};

struct State {
  std::mutex mutex;
  std::vector<Record> records;
  std::size_t total = 0;
  std::size_t warnings = 0;
  std::size_t errors = 0;
  std::size_t renders = 0;
  std::size_t scroll = 0;
  bool errors_only = false;
  bool paused = false;
};

Color LevelColor(const std::string& level) {
  if (level == "ERROR") return Color::RGB(255, 104, 128);
  if (level == "WARN") return Color::RGB(255, 190, 92);
  if (level == "OK") return Color::RGB(92, 224, 171);
  if (level == "RENDER") return Color::RGB(190, 151, 255);
  return Color::RGB(89, 220, 235);
}

Element Card(const std::string& label, std::size_t value, Color accent) {
  return vbox({text(std::to_string(value)) | bold | color(accent),
               text(label) | color(Color::RGB(126, 146, 173))}) |
         borderRounded | flex;
}

Element BuildView(State& state) {
  std::vector<Record> records;
  std::size_t total;
  std::size_t warnings;
  std::size_t errors;
  std::size_t renders;
  std::size_t scroll;
  bool errors_only;
  bool paused;
  {
    std::lock_guard<std::mutex> lock(state.mutex);
    records = state.records;
    total = state.total;
    warnings = state.warnings;
    errors = state.errors;
    renders = state.renders;
    scroll = state.scroll;
    errors_only = state.errors_only;
    paused = state.paused;
  }

  Elements log_lines;
  for (const auto& record : records) {
    if (errors_only && record.level != "WARN" && record.level != "ERROR") continue;
    log_lines.push_back(hbox({text(record.level) | bold | color(LevelColor(record.level)) | size(WIDTH, EQUAL, 7),
                              text(record.scope) | color(Color::RGB(190, 151, 255)) | size(WIDTH, EQUAL, 12),
                              text(record.message) | color(Color::RGB(219, 229, 242)) | flex}));
  }
  if (log_lines.empty()) log_lines.push_back(text("Aguardando atividade do estúdio...") | dim);
  const auto begin = log_lines.size() > 14 + scroll ? log_lines.size() - 14 - scroll : 0;
  const auto end = std::min(log_lines.size(), begin + 14);
  Elements visible(log_lines.begin() + static_cast<std::ptrdiff_t>(begin),
                   log_lines.begin() + static_cast<std::ptrdiff_t>(end));

  auto header = hbox({text(" KAMAFEU ") | bold | color(Color::RGB(10, 15, 27)) | bgcolor(Color::RGB(89, 220, 235)),
                      text("  CONSOLE DE ESTÚDIO  ") | bold | color(Color::White),
                      text("│ TAINAH · WORLD · DSP") | color(Color::RGB(126, 146, 173)),
                      filler(), text("FTXUI") | color(Color::RGB(92, 224, 171))});
  auto stats = hbox({Card("EVENTOS", total, Color::RGB(89, 220, 235)),
                     Card("AVISOS", warnings, Color::RGB(255, 190, 92)),
                     Card("ERROS", errors, Color::RGB(255, 104, 128)),
                     Card("RENDERS", renders, Color::RGB(190, 151, 255))});
  auto log_title = errors_only ? "AVISOS E ERROS" : (paused ? "FLUXO DO ESTÚDIO · PAUSADO" : "FLUXO DO ESTÚDIO");
  auto logs = vbox(std::move(visible)) | vscroll_indicator | frame | border;
  auto footer = hbox({text(" q ") | bold | color(Color::RGB(89, 220, 235)), text("sair  ") | dim,
                      text("f ") | bold | color(Color::RGB(89, 220, 235)), text("erros  ") | dim,
                      text("p ") | bold | color(Color::RGB(89, 220, 235)), text("pausar  ") | dim,
                      text("c ") | bold | color(Color::RGB(89, 220, 235)), text("limpar  ") | dim,
                      text("↑/↓ navegar  g ao vivo") | dim});
  return vbox({header | border, stats, vbox({text(log_title) | bold | color(Color::RGB(89, 220, 235)), logs}) | border,
               footer | border}) |
         bgcolor(Color::RGB(10, 15, 27)) | border;
}

using Socket =
#ifdef _WIN32
    SOCKET;
#else
    int;
#endif

void ReadEvents(State& state, ScreenInteractive& screen, std::atomic<bool>& done,
                Socket socket) {
  char buffer[8192];
  while (!done.load()) {
#ifdef _WIN32
    const int length = recv(socket, buffer, sizeof(buffer), 0);
#else
    const int length = static_cast<int>(recv(socket, buffer, sizeof(buffer), 0));
#endif
    if (length <= 0) {
      std::this_thread::sleep_for(std::chrono::milliseconds(30));
      continue;
    }
    std::string line(buffer, static_cast<std::size_t>(length));
    if (line == "STOP") {
      screen.PostEvent(Event::q);
      break;
    }
    std::istringstream input(line);
    Record record;
    if (!std::getline(input, record.level, '\t') || !std::getline(input, record.scope, '\t')) continue;
    std::getline(input, record.message);
    {
      std::lock_guard<std::mutex> lock(state.mutex);
      ++state.total;
      if (record.level == "WARN") ++state.warnings;
      if (record.level == "ERROR") ++state.errors;
      if (record.level == "RENDER" || record.scope.find("Render") != std::string::npos) ++state.renders;
      if (!state.paused) {
        state.records.push_back(std::move(record));
        if (state.records.size() > 500) state.records.erase(state.records.begin());
        state.scroll = 0;
      }
    }
    screen.PostEvent(Event::Custom);
  }
}

Socket OpenSocket(unsigned short parent_port) {
#ifdef _WIN32
  WSADATA data;
  if (WSAStartup(MAKEWORD(2, 2), &data) != 0) return INVALID_SOCKET;
  Socket socket = ::socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
  if (socket == INVALID_SOCKET) return INVALID_SOCKET;
  DWORD timeout_ms = 100;
  setsockopt(socket, SOL_SOCKET, SO_RCVTIMEO,
             reinterpret_cast<const char*>(&timeout_ms), sizeof(timeout_ms));
#else
  Socket socket = ::socket(AF_INET, SOCK_DGRAM, 0);
  if (socket < 0) return -1;
  timeval timeout{0, 100000};
  setsockopt(socket, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout));
#endif
  sockaddr_in local{};
  local.sin_family = AF_INET;
  local.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  local.sin_port = 0;
  if (bind(socket, reinterpret_cast<sockaddr*>(&local), sizeof(local)) != 0) {
#ifdef _WIN32
    closesocket(socket);
#else
    close(socket);
#endif
    return
#ifdef _WIN32
        INVALID_SOCKET;
#else
        -1;
#endif
  }
#ifdef _WIN32
  int length = sizeof(local);
#else
  socklen_t length = sizeof(local);
#endif
  if (getsockname(socket, reinterpret_cast<sockaddr*>(&local), &length) != 0) return
#ifdef _WIN32
      INVALID_SOCKET;
#else
      -1;
#endif
  sockaddr_in parent{};
  parent.sin_family = AF_INET;
  parent.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  parent.sin_port = htons(parent_port);
  const auto ready = std::string("READY\t") + std::to_string(ntohs(local.sin_port));
  sendto(socket, ready.data(), static_cast<int>(ready.size()), 0,
         reinterpret_cast<sockaddr*>(&parent), sizeof(parent));
  return socket;
}

void CloseSocket(Socket socket) {
#ifdef _WIN32
  closesocket(socket);
  WSACleanup();
#else
  close(socket);
#endif
}

}  // namespace

int main(int argc, char** argv) {
  if (argc != 2) return 2;
  const auto parent_port = std::strtoul(argv[1], nullptr, 10);
  if (parent_port == 0 || parent_port > 65535) return 2;
  const Socket socket = OpenSocket(static_cast<unsigned short>(parent_port));
#ifdef _WIN32
  if (socket == INVALID_SOCKET) return 3;
#else
  if (socket < 0) return 3;
#endif
  State state;
  auto screen = ScreenInteractive::Fullscreen();
  std::atomic<bool> done{false};
  std::thread reader(ReadEvents, std::ref(state), std::ref(screen), std::ref(done), socket);
  auto component = Renderer([&] { return BuildView(state); });
  component |= CatchEvent([&](Event event) {
    if (event == Event::Custom) {
      return true;
    }
    if (event == Event::Escape || event == Event::q) {
      screen.ExitLoopClosure()();
      return true;
    }
    std::lock_guard<std::mutex> lock(state.mutex);
    if (event == Event::Character("f")) state.errors_only = !state.errors_only;
    else if (event == Event::Character("p")) state.paused = !state.paused;
    else if (event == Event::Character("c")) state.records.clear();
    else if (event == Event::Character("g")) state.scroll = 0;
    else if (event == Event::ArrowUp) ++state.scroll;
    else if (event == Event::ArrowDown && state.scroll > 0) --state.scroll;
    else return false;
    return true;
  });
  screen.Loop(component);
  done.store(true);
  if (reader.joinable()) reader.join();
  CloseSocket(socket);

  // Explicitly disable all xterm mouse reporting modes and reset terminal state.
  // This guarantees that even if FTXUI's internal teardown didn't fully flush,
  // mouse motion events (e.g. \033[<35;...M) won't leak into the user's shell.
  std::cout << "\033[?1000l"   // Disable VT200 mouse tracking
            << "\033[?1002l"   // Disable button-event mouse tracking
            << "\033[?1003l"   // Disable any-event / motion mouse tracking
            << "\033[?1005l"   // Disable UTF-8 mouse mode
            << "\033[?1006l"   // Disable SGR extended mouse mode
            << "\033[?1015l"   // Disable urxvt extended mouse mode
            << "\033[?25h"     // Show cursor
            << "\033[?1049l"   // Restore primary screen buffer
            << std::flush;

  return 0;
}
