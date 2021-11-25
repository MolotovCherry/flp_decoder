#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::io::{Cursor, Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};

use num_enum::TryFromPrimitive;
use num_enum::IntoPrimitive;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct FLP {
    chunkID: String,
    length: u32,
    format: Header_Format,
    nChannels: u16,
    beatDiv: u16,
    data: Data
}

#[derive(Debug)]
pub struct Data {
    chunkId: String,
    length: u32,
    events: Vec<Event>
}

#[derive(Debug)]
pub struct Event {
    id: u8,
    length: u64, // length of event in bytes
    eventType: EventType,
    data: EventData,
    offset: u64
}

#[derive(Debug)]

pub enum EventType {
    BYTE(FLP_Event),
    WORD(FLP_Event),
    DWORD(FLP_Event),
    TEXT(FLP_Event)
}

#[derive(Debug)]

pub enum EventData {
    DATA(Vec<u8>),
    TEXT(String),
    TEXTBIN(Vec<u8>)
}

impl FLP {
    pub fn read<F: AsRef<str>>(location: F) -> FLP {
        let data = std::fs::read(std::path::Path::new(location.as_ref())).expect("file not found");

        let mut buffer = Cursor::new(data);
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //
        //  Read the file header
        //
        let mut _flp_chunkID = [0; 4];
        buffer.read_exact(&mut _flp_chunkID).unwrap();
        let flp_chunkID = String::from(std::str::from_utf8(&_flp_chunkID).unwrap());

        let flp_length = buffer.read_u32::<LittleEndian>().unwrap();
        let flp_format = buffer.read_u16::<LittleEndian>().unwrap();
        let flp_nChannels = buffer.read_u16::<LittleEndian>().unwrap();
        let flp_beatDiv = buffer.read_u16::<LittleEndian>().unwrap();

        //
        //  Read the data section
        //
        let mut _data_chunkID = [0; 4];
        buffer.read_exact(&mut _data_chunkID).unwrap();
        let data_chunkID = String::from(std::str::from_utf8(&_data_chunkID).unwrap());

        let data_length = buffer.read_u32::<LittleEndian>().unwrap();

        //
        // Process Events
        //
        let mut events: Vec<Event> = vec![];

        let event_start = buffer.position();
        loop {
            let buf_pos = buffer.position();
            let event_id = buffer.read_u8().unwrap();

            let mut event_length: u64 = 0;
            let event_data;
            let event_type = match event_id {
                0..=63 => {
                    let mut buf: [u8; 1] = [0; 1];
                    buffer.read_exact(&mut buf).unwrap();
                    event_data = EventData::DATA(buf.to_vec());
                    event_length = 1;
                    EventType::BYTE(FLP_Event::try_from(event_id).unwrap_or(FLP_Event::FLP_Unknown))
                },

                64..=127 => {
                    let mut buf: [u8; 2] = [0; 2];
                    buffer.read_exact(&mut buf).unwrap();
                    event_data = EventData::DATA(buf.to_vec());
                    event_length = 2;
                    EventType::WORD(FLP_Event::try_from(event_id).unwrap_or(FLP_Event::FLP_Unknown))
                },

                128..=191 => {
                    let mut buf: [u8; 4] = [0; 4];
                    buffer.read_exact(&mut buf).unwrap();
                    event_data = EventData::DATA(buf.to_vec());
                    event_length = 4;
                    EventType::DWORD(FLP_Event::try_from(event_id).unwrap_or(FLP_Event::FLP_Unknown))
                },

                // variable length
                192..=255 => {
                    let mut size: u64 = 0;

                    let mut byte;

                    loop {
                        byte = buffer.read_u8().unwrap();
                        // grab and add first 7 bits of number to total
                        size += (0x7F & byte) as u64;

                        // check if bit 7 is off
                        if (0x80 & byte) == 0 {
                            break;
                        }
                    }

                    // old flp code, no longer relevant
                    /*loop {
                        size >>= 7;
                        num_bytes += 1;
                        if size == 0 {
                            break;
                        }
                    }*/

                    event_length = size;

                    let mut buf: Vec<u8> = vec![];
                    for _ in 0..size {
                        buf.push(buffer.read_u8().unwrap());
                    }

                    let mut newBuf = buf.clone();
                    newBuf.drain_filter(|x| *x == 0u8);
                    let test_string = std::str::from_utf8(&*newBuf);

                    event_data = match test_string {
                        Ok(v) => {
                            EventData::TEXT(v.to_owned())
                        },
                        Err(_) => EventData::TEXTBIN(buf)
                    };

                    EventType::TEXT(FLP_Event::try_from(event_id).unwrap_or(FLP_Event::FLP_Unknown))
                }
            };

            let event = Event {
                id: event_id,
                length: event_length,
                eventType: event_type,
                data: event_data,
                offset: buf_pos
            };

            events.push(event);

            // check if data end // removing 2 dwords = 8 bytes, because they must be subtracted
            if buffer.position() >= (event_start - 8) + data_length as u64 {
                break;
            }
        }

        let data = Data {
            chunkId: data_chunkID,
            length: data_length,
            events
        };

        FLP {
            chunkID: flp_chunkID,
            length: flp_length,
            format: Header_Format::try_from(flp_format as i32).unwrap(),
            nChannels: flp_nChannels,
            beatDiv: flp_beatDiv,
            data,
        }
    }
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(i32)]
enum Header_Format {
    FLP_Format_None          = -1,      // temporary
    FLP_Format_Song          = 0,       // full project
    FLP_Format_Score         = 0x10,    // score
    FLP_Format_Auto          = (Header_Format::FLP_Format_Score as u8 + 8) as i32, // automation
    FLP_Format_ChanState     = 0x20,    // channel
    FLP_Format_PlugState     = 0x30,    // plugin
    FLP_Format_PlugState_Gen = 0x31,
    FLP_Format_PlugState_FX  = 0x32,
    FLP_Format_MixerState    = 0x40,    // mixer track
    FLP_Format_Patcher       = 0x50     // special: tells to Patcherize (internal)
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
enum Plugin_Flags {
    Plug_Visible      = 1,         // editor is visible or not
    Plug_Disabled     = 2,         // obsolete
    Plug_Detached     = 4,         // editor is detached
    Plug_Maximized    = 8,         // editor is maximized
    Plug_Generator    = 16,        // plugin is a generator (can be loaded into a channel)
    Plug_SD           = 32,        // smart disable option is on
    Plug_TP           = 64,        // threaded processing option is on
    Plug_Demo         = 128,       // saved with a demo version
    Plug_HideSettings = 1 << 8,    // editor is in compact mode
    Plug_Captionized  = 1 << 9,    // editor is captionized
    Plug_DX           = 1 << 16,   // indicates the plugin is a DirectX plugin (obsolete)
    Plug_EditorSize   = 2 << 16,   // editor size is specified (obsolete)
    Plug_EditorFlags  = (
        Plugin_Flags::Plug_Visible as u32 |
        Plugin_Flags::Plug_Detached as u32 |
        Plugin_Flags::Plug_Maximized as u32 |
        Plugin_Flags::Plug_HideSettings as u32 |
        Plugin_Flags::Plug_Captionized as u32
    ) as u32
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
enum FLP_Event {
    FLP_ChanEnabled            = 0,
    FLP_NoteOn                 = 1,      // +pos
    FLP_ChanVol                = 2,      // obsolete
    FLP_ChanPan                = 3,      // obsolete
    FLP_MIDIChan               = 4,
    FLP_MIDINote               = 5,
    FLP_MIDIPatch              = 6,
    FLP_MIDIBank               = 7,
    FLP_LoopActive             = 9,
    FLP_ShowInfo               = 10,
    FLP_Shuffle                = 11,
    FLP_MainVol                = 12,     // obsolete
    FLP_FitToSteps             = 13,     // obsolete byte version
    FLP_Pitchable              = 14,     // obsolete
    FLP_Zipped                 = 15,
    FLP_Delay_Flags            = 16,     // obsolete
    FLP_TimeSig_Num            = 17,
    FLP_TimeSig_Beat           = 18,
    FLP_UseLoopPoints          = 19,
    FLP_LoopType               = 20,
    FLP_ChanType               = 21,
    FLP_TargetFXTrack          = 22,
    FLP_PanVolTab              = 23,     // log vol & circular pan tables
    FLP_nStepsShown            = 24,     // obsolete
    FLP_SSLength               = 25,     // +length
    FLP_SSLoop                 = 26,
    FLP_FXProps                = 27,     // FlipY, ReverseStereo, etc
    FLP_Registered             = 28,     // reg version
    FLP_APDC                   = 29,
    FLP_TruncateClipNotes      = 30,
    FLP_EEAutoMode             = 31,

    FLP_NewChan                = 64,
    FLP_NewPat                 = 64+1,   // +PatNum (word)
    FLP_Tempo                  = 64+2,   // obsolete, replaced by FLP_FineTempo
    FLP_CurrentPatNum          = 64+3,
    FLP_PatData                = 64+4,
    FLP_FX                     = 64+5,
    FLP_FXFlags                = 64+6,
    FLP_FXCut                  = 64+7,
    FLP_DotVol                 = 64+8,
    FLP_DotPan                 = 64+9,
    FLP_FXPreamp               = 64+10,
    FLP_FXDecay                = 64+11,
    FLP_FXAttack               = 64+12,
    FLP_DotNote                = 64+13,
    FLP_DotPitch               = 64+14,
    FLP_DotMix                 = 64+15,
    FLP_MainPitch              = 64+16,
    FLP_RandChan               = 64+17,  // obsolete
    FLP_MixChan                = 64+18,  // obsolete
    FLP_FXRes                  = 64+19,
    FLP_OldSongLoopPos         = 64+20,  // obsolete
    FLP_FXStDel                = 64+21,
    FLP_FX3                    = 64+22,
    FLP_DotFRes                = 64+23,
    FLP_DotFCut                = 64+24,
    FLP_ShiftTime              = 64+25,
    FLP_LoopEndBar             = 64+26,
    FLP_Dot                    = 64+27,
    FLP_DotShift               = 64+28,
    FLP_Tempo_Fine             = 64+29,  // obsolete, replaced by FLP_FineTempo
    FLP_LayerChan              = 64+30,
    FLP_FXIcon                 = 64+31,
    FLP_DotRel                 = 64+32,
    FLP_SwingMix               = 64+33,

    FLP_PluginColor            = 128,
    FLP_PLItem                 = 128+1,  // Pos (word) +PatNum (word) (obsolete)
    FLP_Echo                   = 128+2,
    FLP_FXSine                 = 128+3,
    FLP_CutCutBy               = 128+4,
    FLP_WindowH                = 128+5,
    FLP_MiddleNote             = 128+7,
    FLP_Reserved               = 128+8,  // may contain an invalid version info
    FLP_MainResCut             = 128+9,  // obsolete
    FLP_DelayFRes              = 128+10,
    FLP_Reverb                 = 128+11,
    FLP_StretchTime            = 128+12,
    FLP_SSNote                 = 128+13, // SimSynth patch middle note (obsolete)
    FLP_FineTune               = 128+14,
    FLP_SampleFlags            = 128+15,
    FLP_LayerFlags             = 128+16,
    FLP_ChanFilterNum          = 128+17,
    FLP_CurrentFilterNum       = 128+18,
    FLP_FXOutChanNum           = 128+19, // FX track output channel
    FLP_NewTimeMarker          = 128+20, // + Time & Mode in higher bits
    FLP_FXColor                = 128+21,
    FLP_PatColor               = 128+22,
    FLP_PatAutoMode            = 128+23, // obsolete
    FLP_SongLoopPos            = 128+24,
    FLP_AUSmpRate              = 128+25,
    FLP_FXInChanNum            = 128+26, // FX track input channel
    FLP_PluginIcon             = 128+27,
    FLP_FineTempo              = 128+28,

    FLP_Text                   = 192,    // +Length (VarLengthInt) +Text (Null Term. AnsiString)
    FLP_Text_PatName           = 192+1,
    FLP_Text_Title             = 192+2,
    FLP_Text_Comment           = 192+3,
    FLP_Text_SampleFileName    = 192+4,
    FLP_Text_URL               = 192+5,
    FLP_Text_CommentRTF        = 192+6,  // comments in Rich Text format
    FLP_Version                = 192+7,
    FLP_RegName                = 192+8,  // since 1.3.9 the (scrambled) reg name is stored in the FLP
    FLP_Text_DefPluginName     = 192+9,
    //FLP_Text_CommentRTF_SC   = FLP_Text+10;  // new comments in Rich Text format (obsolete)
    FLP_Text_ProjDataPath      = 192+10,
    FLP_Text_PluginName        = 192+11, // plugin's name
    FLP_Text_FXName            = 192+12, // FX track name
    FLP_Text_TimeMarker        = 192+13, // time marker name
    FLP_Text_Genre             = 192+14,
    FLP_Text_Author            = 192+15,
    FLP_MIDICtrls              = 192+16,
    FLP_Delay                  = 192+17,
    FLP_TS404Params            = 192+18,
    FLP_DelayLine              = 192+19, // obsolete
    FLP_NewPlugin              = 192+20, // new VST or DirectX plugin
    FLP_PluginParams           = 192+21,
    FLP_Reserved2              = 192+22, // used once for testing
    FLP_ChanParams             = 192+23, // block of various channel params (can grow)
    FLP_CtrlRecChan            = 192+24, // automated controller events
    FLP_PLSel                  = 192+25, // selection in playlist
    FLP_Envelope               = 192+26,
    FLP_ChanLevels             = 192+27, // pan, vol, pitch, filter, filter type
    FLP_ChanFilter             = 192+28, // cut, res, type (obsolete)
    FLP_ChanPoly               = 192+29, // max poly, poly slide, monophonic
    FLP_NoteRecChan            = 192+30, // automated note events
    FLP_PatCtrlRecChan         = 192+31, // automated ctrl events per pattern
    FLP_PatNoteRecChan         = 192+32, // automated note events per pattern
    FLP_InitCtrlRecChan        = 192+33, // init values for automated events
    FLP_RemoteCtrl_MIDI        = 192+34, // remote control entry (MIDI)
    FLP_RemoteCtrl_Int         = 192+35, // remote control entry (internal)
    FLP_Tracking               = 192+36, // vol/kb tracking
    FLP_ChanOfsLevels          = 192+37, // levels offset
    FLP_Text_RemoteCtrlFormula = 192+38, // remote control entry formula
    FLP_Text_ChanFilter        = 192+39,
    FLP_RegBlackList           = 192+40, // black list of reg codes
    FLP_PLRecChan              = 192+41, // playlist
    FLP_ChanAC                 = 192+42, // channel articulator
    FLP_FXRouting              = 192+43,
    FLP_FXParams               = 192+44,
    FLP_ProjectTime            = 192+45,
    FLP_PLTrackInfo            = 192+46,
    FLP_Text_PLTrackName       = 192+47,

    FLP_Unknown
}
