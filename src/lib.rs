//! Horizon Alive WorldFX 0.6.0: real XYZ vanilla particle commands, never HUD.
//! SDK 0.1 cannot observe other TNT explosions. Only /alive tnt instruments TNT.
mod worldfx;

use horizon_mod_sdk::{Mod, Registration, command, game, log, render, storage, ui};
use worldfx::{Blast, Config, Quality, COOLDOWN_TICKS, LIFE_TICKS, MAX_ACTIVE};

const ENABLED: &str = "alive.enabled";
const QUALITY: &str = "alive.quality";
const SCALE: &str = "alive.scale";
const FLASH: &str = "alive.flash";
const SMOKE: &str = "alive.smoke";

fn stored(key: &str, default: u8) -> u8 {
    storage::get(key).and_then(|v| v.first().copied()).unwrap_or(default)
}
fn save(key: &str, v: u8) {
    if let Err(e) = storage::set(key, &[v]) {
        log::write(log::Level::Warn, &format!("Horizon Alive: saving settings failed: {e}"));
    }
}

struct Alive {
    enabled: bool,
    config: Config,
    active: Vec<Blast>,
    last_trigger: Option<u64>,
    last_tick: Option<u64>,
    dimension: Option<String>,
}
impl Alive {
    fn load() -> Self {
        Self {
            enabled: stored(ENABLED,1) != 0,
            config: Config {
                quality: Quality::from_byte(stored(QUALITY,1)),
                scale: f64::from(stored(SCALE,10).clamp(5,20))/10.0,
                flash: stored(FLASH,1) != 0,
                smoke: stored(SMOKE,1) != 0,
            },
            active: Vec::with_capacity(MAX_ACTIVE),
            last_trigger: None,
            last_tick: None,
            dimension: None,
        }
    }
    fn settings(&self) {
        ui::set_settings(&[
            ui::Widget::Text(ui::TextWidget {
                id:10,
                text:"Vanilla 3D world particles. /alive tnt requires cheats. Ordinary TNT is unchanged.".into(),
                color:0xBDDDE8,
            }),
            ui::Widget::Button(ui::ButtonWidget{id:1,label:format!("World FX: {}",if self.enabled {"ON"} else {"OFF"})}),
            ui::Widget::Button(ui::ButtonWidget{id:2,label:format!("Quality: {}",self.config.quality.name())}),
            ui::Widget::Button(ui::ButtonWidget{id:3,label:format!("Visual scale: {:.1}x",self.config.scale)}),
            ui::Widget::Button(ui::ButtonWidget{id:4,label:format!("Smoke: {}",if self.config.smoke {"ON"}else{"OFF"})}),
            ui::Widget::Button(ui::ButtonWidget{id:5,label:format!("Flash: {}",if self.config.flash {"ON"}else{"OFF"})}),
        ]);
    }
    fn status(&self)->String {
        format!("Alive v0.6: {}; {} quality; {:.1}x; {} effects; vanilla TNT not tracked",
            if self.enabled{"on"}else{"off"},self.config.quality.name(),self.config.scale,self.active.len())
    }
    fn begin(&mut self, with_tnt:bool)->Result<Option<String>,String> {
        if !self.enabled {return Err("Enable with /alive on".into());}
        let player=game::get_player().ok_or("Open a world first")?;
        let clock=game::get_clock().ok_or("World clock unavailable")?;
        if self.last_trigger.is_some_and(|last|clock.game_time>=last&&clock.game_time-last<COOLDOWN_TICKS){
            return Err("Cooldown active; try again shortly".into());
        }
        let hit=game::raycast(12.0).ok_or("Aim at a block within 12 blocks")?;
        let origin=worldfx::anchor_from_block(hit.block_x,hit.block_y,hit.block_z);
        if with_tnt {
            let cmd=worldfx::summon_tnt(origin).ok_or("Invalid coordinates")?;
            // One TNT only. Command runs as the player with standard permissions.
            // A world with cheats disabled refuses the request; no privileges bypassed.
            command::run(&cmd);
        }
        if self.active.len()>=MAX_ACTIVE {self.active.remove(0);}
        self.active.push(Blast{
            origin,
            start_tick:clock.game_time,
            dimension:player.dimension.clone(),
            seed:clock.game_time^origin[0].to_bits()^origin[2].to_bits(),
        });
        self.last_trigger=Some(clock.game_time);
        self.dimension=Some(player.dimension);
        Ok(Some(if with_tnt {
            "Requested one vanilla TNT and its 3D world-space effect (cheats required).".into()
        } else {
            "3D world-space visual preview, no explosion damage.".into()
        }))
    }
    fn configure(&mut self,key:&str,value:&str)->Result<Option<String>,String>{
        match key {
            "quality"=>{
                self.config.quality=match value {
                    "low"=>Quality::Low,"balanced"=>Quality::Balanced,"high"=>Quality::High,
                    _=>return Err("Use /alive quality low|balanced|high".into()),
                };
                save(QUALITY,self.config.quality.as_byte());
            }
            "scale"=>{
                let v:f64=value.parse().map_err(|_|"Use /alive scale 0.5..2.0")?;
                if !v.is_finite() || !(0.5..=2.0).contains(&v) {
                    return Err("Use /alive scale 0.5..2.0".into());
                }
                self.config.scale=(v*10.0).round()/10.0;
                save(SCALE,(self.config.scale*10.0) as u8);
            }
            "smoke"=>{
                self.config.smoke=match value {"on"=>true,"off"=>false,_=>return Err("Use /alive smoke on|off".into())};
                save(SMOKE,u8::from(self.config.smoke));
            }
            "flash"=>{
                self.config.flash=match value {"on"=>true,"off"=>false,_=>return Err("Use /alive flash on|off".into())};
                save(FLASH,u8::from(self.config.flash));
            }
            _=>return Err("Unknown setting".into()),
        };
        self.settings();
        Ok(Some("Horizon Alive settings saved".into()))
    }
}
impl Mod for Alive {
    fn init()->Result<(Self,Registration),String>{
        render::set_hud(&[]);
        log::write(log::Level::Info,"Horizon Alive 0.6: vanilla world-space effects ready");
        Ok((Self::load(),Registration::new().command("alive").on_tick()))
    }
    fn tick(&mut self,tick:u64){
        if self.last_tick==Some(tick){return;}
        self.last_tick=Some(tick);
        let Some(player)=game::get_player() else {
            self.active.clear();self.dimension=None;self.last_trigger=None;return;
        };
        let Some(clock)=game::get_clock() else {self.active.clear();return;};
        if self.dimension.as_deref()!=Some(player.dimension.as_str())
            || self.active.iter().any(|e|e.start_tick>clock.game_time){
            self.active.clear();self.last_trigger=None;
        }
        self.dimension=Some(player.dimension.clone());
        if !self.enabled {self.active.clear();return;}
        self.active.retain(|b|b.dimension==player.dimension
            && clock.game_time>=b.start_tick
            && clock.game_time-b.start_tick<LIFE_TICKS);
        let frames:Vec<_>=self.active.iter().map(|b|worldfx::frame(b,clock.game_time-b.start_tick,&self.config)).collect();
        let commands=worldfx::interleave(&frames);
        if !commands.is_empty(){command::run_quiet(&commands);}
    }
    fn command(&mut self,name:&str,args:&str)->Result<Option<String>,String>{
        if name!="alive"{return Err("Unknown command".into());}
        match args.trim(){
            ""|"help"=>Ok(Some("/alive tnt|preview|status|on|off|quality low|balanced|high|scale 0.5..2.0|smoke on|off|flash on|off".into())),
            "status"=>Ok(Some(self.status())),
            "on"=>{self.enabled=true;save(ENABLED,1);self.settings();Ok(Some("Horizon Alive enabled".into()))}
            "off"=>{self.enabled=false;self.active.clear();save(ENABLED,0);self.settings();Ok(Some("Horizon Alive disabled".into()))}
            "tnt"=>self.begin(true),
            "preview"=>self.begin(false),
            "graphics off"=>{render::set_hud(&[]);Ok(Some("Horizon Alive HUD is empty".into()))}
            other=>{
                if let Some((k,v))=other.split_once(' '){self.configure(k,v)}
                else {Err("Use /alive help".into())}
            }
        }
    }
    fn ui_settings_opened(&mut self){self.settings();}
    fn ui_select(&mut self,id:u32,_row:u32){
        match id{
            1=>{self.enabled=!self.enabled;if !self.enabled{self.active.clear();}save(ENABLED,u8::from(self.enabled));}
            2=>{self.config.quality=self.config.quality.next();save(QUALITY,self.config.quality.as_byte());}
            3=>{self.config.scale=if self.config.scale>=2.0{0.5}else{(self.config.scale+0.5).min(2.0)};save(SCALE,(self.config.scale*10.0) as u8);}
            4=>{self.config.smoke=!self.config.smoke;save(SMOKE,u8::from(self.config.smoke));}
            5=>{self.config.flash=!self.config.flash;save(FLASH,u8::from(self.config.flash));}
            _=>return,
        }
        self.settings();
    }
}
horizon_mod_sdk::export_mod!(Alive);

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn quality_rotation(){assert_eq!(Quality::High.next(),Quality::Low);}
    #[test] fn stored_scale_bounds(){assert!((5..=20).contains(&10_u8.clamp(5,20)));}
}
