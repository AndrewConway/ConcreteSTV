// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use clap::Parser;
use std::path::PathBuf;
use std::fs::File;

#[derive(Parser)]
#[clap(version = "0.1", author = "Andrew Conway", name="ConcreteSTV")]
/// Convert a .soi or .soc file from preflib to ConcreteSTV .stv format. See https://www.preflib.org/.
/// Note that this application will not do a good job of interpreting the name in preflib to extract electorate, year, authority, copyright, etc., and will assume all votes are BTL.
struct Opts {
    /// The name of the .soi or .soc preflib file to convert to ConcreteSTV format
    #[clap(value_parser)]
    file : PathBuf,

    /// An optional output file. If not specified, the input file name is used with the extension changed to .stv
    /// It is strongly recommended that this be used as stdout is also used for other information.
    #[clap(short, long,value_parser)]
    out : Option<PathBuf>,

}



fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();
    let data = preflib::parse(&opt.file)?;
    let out_path = if let Some(path) = &opt.out { path.clone() } else {
        let mut path = PathBuf::from(opt.file.file_name().unwrap_or_default());
        path.set_extension("stv");
        path
    };
    let out = File::create(&out_path)?;
    serde_json::to_writer(out,&data)?;
    Ok(())
}
