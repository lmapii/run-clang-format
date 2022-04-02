// https://stackoverflow.com/questions/21011330/how-do-i-invoke-a-system-command-and-capture-its-output
// https://stackoverflow.com/questions/49218599/write-to-child-process-stdin-in-rust/49597789#49597789

use std::io;
use std::path;
use std::process;

pub struct Runner {
    cmd: path::PathBuf,
}

impl Runner {
    pub fn new<P>(path: P) -> Runner
    where
        P: AsRef<path::Path>,
    {
        let cmd = path::PathBuf::from(path.as_ref());
        Runner { cmd }
    }

    fn eval_status(status: process::ExitStatus) -> Result<(), io::Error> {
        match status.code() {
            Some(code) if code == 0 => (),
            Some(code) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Process terminated with code {}", code),
                ));
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Process terminated by signal",
                ))
            }
        };
        Ok(())
    }

    pub fn get_version(&self) -> Result<String, io::Error> {
        let cmd = process::Command::new(self.cmd.as_path())
            .arg("--version")
            .output()?;

        if let Err(err) = Runner::eval_status(cmd.status) {
            log::error!(
                "Execution failed:\n{}",
                String::from_utf8_lossy(&cmd.stderr)
            );
            return Err(err);
        }

        // example output of clang-format:
        // clang-format version 4.0.0 (tags/checker/checker-279)
        let stdout = String::from_utf8_lossy(&cmd.stdout);

        let re = regex::Regex::new(r".*version ([\d]+)\.([\d]+)\.([\d]+).*").unwrap();
        let caps = re.captures(&stdout).ok_or(io::Error::new(
            io::ErrorKind::Other,
            "Failed to match version",
        ))?;

        Ok(format!("{}.{}.{}", &caps[1], &caps[2], &caps[3]))
    }

    pub fn format<P>(&self, file: P) -> Result<(), io::Error>
    where
        P: AsRef<path::Path>,
    {
        // execute clang-format to edit in place, using style file
        let cmd = process::Command::new(self.cmd.as_path())
            .arg(file.as_ref().as_os_str())
            .arg("-fallback-style=none")
            .arg("-style=file")
            .arg("-i")
            .output()?;

        if let Err(err) = Runner::eval_status(cmd.status) {
            let stderr = String::from_utf8_lossy(&cmd.stderr);

            if stderr.len() != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("{}\n---\n{}", err, stderr),
                ));
            }
            return Err(err);
        }
        Ok(())
    }
}

impl Clone for Runner {
    fn clone(&self) -> Runner {
        Runner {
            cmd: path::PathBuf::from(self.cmd.as_path()),
        }
    }
}
