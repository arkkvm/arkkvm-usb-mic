use std::ffi::{CString, c_void};
use std::ptr;

use libc::{c_char, c_int, size_t, uint8_t};
use rockchip_mpi_sys::{
    AIO_ATTR_S, AUDIO_BIT_WIDTH_E, AUDIO_FRAME_S, AUDIO_SAMPLE_RATE_E, AUDIO_SOUND_MODE_E, MB_BLK,
    MB_EXT_CONFIG_S, RK_MPI_AO_ClearChnBuf, RK_MPI_AO_Disable, RK_MPI_AO_DisableChn,
    RK_MPI_AO_Enable, RK_MPI_AO_EnableChn, RK_MPI_AO_EnableReSmp, RK_MPI_AO_SendFrame,
    RK_MPI_AO_SetPubAttr, RK_MPI_MB_ReleaseMB, RK_MPI_SYS_CreateMB, RK_MPI_SYS_Init, RK_S32, RK_U8,
    RK_U32, RK_U64,
};
use tracing::{debug, error, info, warn};

pub const RK_SUCCESS: i32 = 0;
pub const AUDIO_SOUND_MODE_MONO: u32 = 0;
pub const AUDIO_SOUND_MODE_STEREO: u32 = 1;
pub const AUDIO_SOUND_MODE_BUTT: u32 = 9;
pub const AUDIO_BIT_WIDTH_16: u32 = 1;
pub const RK_FALSE: u32 = 0;
pub const RK_TRUE: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum AudioOutputError {
    Success = 0,
    ErrorInit = -1,
    ErrorSystem = -2,
    ErrorMemory = -3,
    ErrorInvalidHandle = -4,
    ErrorOutput = -5,
    ErrorParam = -6,
    ErrorFile = -7,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct AudioOutputConfig {
    pub card_name: Option<CString>,
    pub sample_rate: u32,
    pub channels: u32,
    pub format: Option<CString>,
}

impl Default for AudioOutputConfig {
    fn default() -> Self {
        AudioOutputConfig {
            card_name: Some(CString::new("hw:1,0").unwrap()),
            sample_rate: 48000,
            channels: 2,
            format: Some(CString::new("S16").unwrap()),
        }
    }
}
#[derive(Debug, Clone)]
pub struct AudioOutputContext {
    ao_dev_id: i32,
    ao_chn_id: i32,
    initialized: bool,
    time_stamp: u64,
    config: AudioOutputConfig,
}

impl AudioOutputContext {
    pub fn new() -> Self {
        AudioOutputContext {
            ao_dev_id: 1,
            ao_chn_id: 0,
            initialized: false,
            time_stamp: 0,
            config: AudioOutputConfig::default(),
        }
    }
}

pub struct AudioProcessorC {
    context: Option<AudioOutputContext>,
}

impl AudioProcessorC {
    pub fn new() -> Self {
        AudioProcessorC { context: None }
    }

    pub fn set_config(&mut self, config: AudioOutputConfig) {
        if let Some(ctx) = &mut self.context {
            ctx.config = config;
        } else {
            let mut ctx = AudioOutputContext::new();
            ctx.config = config;
            self.context = Some(ctx);
        }
    }

    fn find_sound_mode(channels: u32) -> AUDIO_SOUND_MODE_E {
        match channels {
            1 => AUDIO_SOUND_MODE_MONO,
            2 => AUDIO_SOUND_MODE_STEREO,
            _ => AUDIO_SOUND_MODE_BUTT,
        }
    }

    pub fn init(&mut self) -> AudioOutputError {
        unsafe {
            if RK_MPI_SYS_Init() != RK_SUCCESS {
                return AudioOutputError::ErrorSystem;
            }
            // Create or clone context
            let mut context = match &self.context {
                Some(ctx) => ctx.clone(),
                None => AudioOutputContext::new(),
            };
            let mut ao_attr: AIO_ATTR_S = std::mem::zeroed();

            let card_name = match &context.config.card_name {
                Some(name) => name.as_ptr(),
                None => CString::new("hw:1,0").unwrap().as_ptr(),
            };
            let card_name_str = card_name as *const c_char;
            let max_len = std::mem::size_of_val(&ao_attr.u8CardName) - 1;
            let copied = std::cmp::min(libc::strlen(card_name_str) as usize, max_len);
            std::ptr::copy_nonoverlapping(
                card_name_str,
                ao_attr.u8CardName.as_mut_ptr() as *mut c_char,
                copied,
            );
            ao_attr.u8CardName[copied] = 0;

            ao_attr.soundCard.channels = context.config.channels;
            ao_attr.soundCard.sampleRate = context.config.sample_rate;
            ao_attr.soundCard.bitWidth = AUDIO_BIT_WIDTH_16;

            ao_attr.enBitwidth = AUDIO_BIT_WIDTH_16;
            ao_attr.enSamplerate = context.config.sample_rate as AUDIO_SAMPLE_RATE_E;

            let sound_mode = Self::find_sound_mode(context.config.channels);
            if sound_mode == AUDIO_SOUND_MODE_BUTT {
                return AudioOutputError::ErrorParam;
            }
            ao_attr.enSoundmode = sound_mode;
            ao_attr.u32FrmNum = 4;
            ao_attr.u32PtNumPerFrm = 1920 * 2;
            ao_attr.u32EXFlag = 0;
            ao_attr.u32ChnCnt = context.config.channels as u32;
            if RK_MPI_AO_SetPubAttr(context.ao_dev_id, &ao_attr) != RK_SUCCESS
                || RK_MPI_AO_Enable(context.ao_dev_id) != RK_SUCCESS
            {
                return AudioOutputError::ErrorInit;
            }
            if RK_MPI_AO_EnableChn(context.ao_dev_id, context.ao_chn_id) != RK_SUCCESS {
                RK_MPI_AO_Disable(context.ao_dev_id);
                return AudioOutputError::ErrorInit;
            }
            if RK_MPI_AO_EnableReSmp(
                context.ao_dev_id,
                context.ao_chn_id,
                context.config.sample_rate as AUDIO_SAMPLE_RATE_E,
            ) != RK_SUCCESS
            {
                return AudioOutputError::ErrorInit;
                // Continue despite error
            }
            context.initialized = true;
            context.time_stamp = 0;
            self.context = Some(context);
            info!("Initializing Audio Output successful");
            AudioOutputError::Success
        }
    }

    pub fn get_context(&mut self) -> AudioOutputContext {
           match self.context {
            Some(ref ctx) => ctx.clone(),
            None => AudioOutputContext::new() 
        }
    }

    pub fn send_data(&self, data: &[u8]) -> AudioOutputError {
        let context  = match  self.context {
            Some(ref ctx) => ctx,
            None => return AudioOutputError::ErrorInvalidHandle,
        };
        unsafe {
           

            if data.is_empty() {
                return AudioOutputError::ErrorParam;
            }

            let mut frame: AUDIO_FRAME_S = std::mem::zeroed();
            frame.u32Len = data.len() as u32;
            frame.enBitWidth = AUDIO_BIT_WIDTH_16;
            frame.enSoundMode = if context.ao_chn_id == 0 {
                AUDIO_SOUND_MODE_STEREO
            } else {
                AUDIO_SOUND_MODE_MONO
            };
            frame.u64TimeStamp = context.time_stamp;
            frame.bBypassMbBlk = RK_FALSE;

            let mut ext_config: MB_EXT_CONFIG_S = std::mem::zeroed();
            ext_config.pOpaque = data.as_ptr() as *mut c_void;
            ext_config.pu8VirAddr = data.as_ptr() as *mut RK_U8;
            ext_config.u64Size = data.len() as u64;

            let mut mb_blk: *mut c_void = ptr::null_mut();

            if RK_MPI_SYS_CreateMB(&mut mb_blk as *mut *mut c_void, &mut ext_config) != RK_SUCCESS {
                return AudioOutputError::ErrorMemory;
            }

            frame.pMbBlk = mb_blk;

            let result = RK_MPI_AO_SendFrame(context.ao_dev_id, context.ao_chn_id, &frame, -1);

            RK_MPI_MB_ReleaseMB(mb_blk);
            // context.time_stamp += 1;

            if result < 0 { AudioOutputError::ErrorOutput } else { AudioOutputError::Success }
        }
    }

    pub fn release(&mut self) -> AudioOutputError {
        unsafe {
            let context = match &mut self.context {
                Some(ctx) => ctx,
                None => return AudioOutputError::ErrorInvalidHandle,
            };

            if context.initialized {
                RK_MPI_AO_ClearChnBuf(context.ao_dev_id, context.ao_chn_id);
                RK_MPI_AO_DisableChn(context.ao_dev_id, context.ao_chn_id);
                RK_MPI_AO_Disable(context.ao_dev_id);
                context.initialized = false;
            }

            self.context = None;
            AudioOutputError::Success
        }
    }
}

impl Drop for AudioProcessorC {
    fn drop(&mut self) {
        let _ = self.release();
    }
}
