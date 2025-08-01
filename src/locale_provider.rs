use std::process::Command;

use crate::data_gobject::DataGObject;
use crate::prelude::*;

pub struct LocaleProvider();

impl LocaleProvider {
    fn path_join<P: AsRef<std::path::Path>, Pa: AsRef<std::path::Path>>(
        base: P,
        rel: Pa,
    ) -> std::path::PathBuf {
        use std::path::Component::{CurDir, Normal, ParentDir, Prefix, RootDir};
        base.as_ref()
            .components()
            .chain(rel.as_ref().components())
            .fold(Vec::new(), |mut path, part| {
                match part {
                    Prefix(_) => {
                        path.clear();
                        path.push(part);
                    }
                    RootDir => {
                        path.retain(|p| matches!(p, Prefix(_)));
                        path.push(part);
                    }
                    CurDir => {}
                    ParentDir => {
                        if !matches!(path.last(), Some(Prefix(_) | RootDir)) {
                            path.pop();
                        }
                    }
                    part @ Normal(_) => path.push(part),
                }
                path
            })
            .into_iter()
            .collect()
    }

    fn get_locales() -> Result<Vec<LanguageRegionEntry>, Box<dyn std::error::Error>> {
        let output = Command::new("locale").arg("-va").output()?;
        let mut locale = None;
        let mut lang = None;
        let mut terr = None;
        let mut locales = Vec::new();

        for line in String::from_utf8(output.stdout)?
            .lines()
            .map(str::trim)
            .chain(std::iter::once(""))
        {
            if line.is_empty() {
                if let Some((locale, lang, terr)) = locale
                    .take()
                    .map(|loc: String| (loc, lang.take(), terr.take()))
                {
                    if locale
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_lowercase())
                    {
                        let lang = lang.map_or_else(
                            || locale.clone(),
                            |lang| {
                                if let Some(terr) = terr {
                                    format!("{lang} ({terr})")
                                } else {
                                    lang
                                }
                            },
                        );
                        locales.push(LanguageRegionEntry {
                            code: locale,
                            display: lang,
                        });
                    }
                }
            }
            if let Some(loc) = line
                .strip_prefix("locale: ")
                .and_then(|l| l.split_once(' ').map(|(a, _)| a))
            {
                locale = Some(loc.to_owned());
            } else if let Some(lan) = line.strip_prefix("language | ") {
                lang = Some(lan.to_owned());
            } else if let Some(ter) = line.strip_prefix("territory | ") {
                terr = Some(ter.to_owned());
            }
        }

        Ok(locales)
    }

    fn get_current_timezone() -> Result<String, Box<dyn std::error::Error>> {
        let p = Self::path_join("/etc", std::fs::read_link("/etc/localtime")?);
        Ok(p.strip_prefix("/etc/zoneinfo")?
            .to_str()
            .ok_or("Invalid characters in timezone")?
            .to_owned())
    }

    fn get_current_locale() -> Result<String, Box<dyn std::error::Error>> {
        std::fs::read_to_string("/etc/locale.conf")?
            .lines()
            .find_map(|line| {
                line.split_once('=')
                    .and_then(|(var, val)| (var == "LANG").then(|| val.to_owned()))
            })
            .ok_or_else(|| "LANG not found".into())
            .map(|s| {
                s.split_once('.')
                    .map(|(loc, cset)| {
                        loc.chars()
                            .chain(std::iter::once('.'))
                            .chain(cset.chars().filter_map(|c| match c {
                                c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
                                _ => None,
                            }))
                            .collect::<String>()
                    })
                    .unwrap_or(s)
            })
    }

    fn get_timezone_display(tz: &str) -> String {
        tz.chars().map(|c| if c == '_' { ' ' } else { c }).collect()
    }

    fn get_timezones() -> Result<Vec<LanguageRegionEntry>, Box<dyn std::error::Error>> {
        let output = Command::new("timedatectl").arg("list-timezones").output()?;
        Ok(String::from_utf8(output.stdout)?
            .lines()
            .map(|tz| (tz, Self::get_timezone_display(tz)).into())
            .collect())
    }

    pub async fn get_timezone_locale_info() -> LanguageRegionData {
        let (tx_lang, rx_lang) = async_channel::bounded(1);
        let (tx_tz, rx_tz) = async_channel::bounded(1);
        std::thread::spawn(move || {
            if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
                let current = match Self::get_current_locale() {
                    Ok(v) => Some(v),
                    Err(e) => {
                        warn!("Error detecting current locale: {e}");
                        Some(String::from("en_US.utf8"))
                    }
                };
                let locales = Self::get_locales()?;
                Ok(tx_lang.send_blocking((current, locales))?)
            })() {
                warn!("Getting locales failed: {e}, using defaults");
                drop(tx_lang);
            }

            if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
                let current = match Self::get_current_timezone() {
                    Ok(v) => Some(v),
                    Err(e) => {
                        warn!("Error detecting current timezone: {e}");
                        Some(String::from("UTC"))
                    }
                };
                let timezones = Self::get_timezones()?;
                Ok(tx_tz.send_blocking((current, timezones))?)
            })() {
                warn!("Getting timezones failed: {e}, using defaults");
            }
        });

        let (current_language, languages) = rx_lang.recv().await.unwrap_or_else(|_| {
            (
                Some(String::from("en_US.utf8")),
                vec![
                    ("ar_AE.utf8", "Arabic (UAE)").into(),
                    ("en_US.utf8", "English (United States)").into(),
                ],
            )
        });

        let (current_timezone, timezones) = rx_tz.recv().await.unwrap_or_else(|_| {
            (
                Some(String::from("UTC")),
                vec![
                    ("Europe/Helsinki", "Europe/Helsinki").into(),
                    ("Asia/Abu_Dhabi", "Asia/Abu Dhabi").into(),
                ],
            )
        });

        LanguageRegionData {
            languages,
            current_language,
            timezones,
            current_timezone,
        }
    }
}

pub struct LanguageRegionEntry {
    pub code: String,
    pub display: String,
}

impl<T: Into<String>, U: Into<String>> From<(T, U)> for LanguageRegionEntry {
    fn from(val: (T, U)) -> Self {
        Self {
            code: val.0.into(),
            display: val.1.into(),
        }
    }
}

impl From<LanguageRegionEntry> for DataGObject {
    fn from(val: LanguageRegionEntry) -> Self {
        Self::new(val.code, val.display)
    }
}

pub struct LanguageRegionData {
    pub languages: Vec<LanguageRegionEntry>,
    pub current_language: Option<String>,
    pub timezones: Vec<LanguageRegionEntry>,
    pub current_timezone: Option<String>,
}
