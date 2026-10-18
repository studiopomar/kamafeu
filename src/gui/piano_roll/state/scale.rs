#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MusicalScale {
    #[default]
    Chromatic,
    Major,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    PentatonicMajor,
    PentatonicMinor,
    Blues,
    Dorian,
    Mixolydian,
}

impl MusicalScale {
    pub const ALL: [MusicalScale; 10] = [
        MusicalScale::Chromatic,
        MusicalScale::Major,
        MusicalScale::NaturalMinor,
        MusicalScale::HarmonicMinor,
        MusicalScale::MelodicMinor,
        MusicalScale::PentatonicMajor,
        MusicalScale::PentatonicMinor,
        MusicalScale::Blues,
        MusicalScale::Dorian,
        MusicalScale::Mixolydian,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            MusicalScale::Chromatic => "Cromática (Livre)",
            MusicalScale::Major => "Maior (Jônio)",
            MusicalScale::NaturalMinor => "Menor Natural (Eólio)",
            MusicalScale::HarmonicMinor => "Menor Harmônica",
            MusicalScale::MelodicMinor => "Menor Melódica",
            MusicalScale::PentatonicMajor => "Pentatônica Maior",
            MusicalScale::PentatonicMinor => "Pentatônica Menor",
            MusicalScale::Blues => "Blues",
            MusicalScale::Dorian => "Dórico",
            MusicalScale::Mixolydian => "Mixolídio",
        }
    }

    pub fn intervals(&self) -> &'static [u8] {
        match self {
            MusicalScale::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            MusicalScale::Major => &[0, 2, 4, 5, 7, 9, 11],
            MusicalScale::NaturalMinor => &[0, 2, 3, 5, 7, 8, 10],
            MusicalScale::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            MusicalScale::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
            MusicalScale::PentatonicMajor => &[0, 2, 4, 7, 9],
            MusicalScale::PentatonicMinor => &[0, 3, 5, 7, 10],
            MusicalScale::Blues => &[0, 3, 5, 6, 7, 10],
            MusicalScale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            MusicalScale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
        }
    }

    pub fn is_in_scale(&self, root_key: u8, midi: u8) -> bool {
        if *self == MusicalScale::Chromatic {
            return true;
        }
        let note_semitone = (midi as i32 - root_key as i32).rem_euclid(12) as u8;
        self.intervals().contains(&note_semitone)
    }

    pub fn is_tonic(&self, root_key: u8, midi: u8) -> bool {
        (midi as i32 - root_key as i32).rem_euclid(12) == 0
    }
}

pub const ROOT_NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
