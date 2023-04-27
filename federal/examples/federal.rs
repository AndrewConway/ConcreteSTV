// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Utility program to test file parsing. Useful only for development.

use federal::FederalRulesPre2021;
use std::fs::File;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::random_util::Randomness;

fn main()  -> anyhow::Result<()> {
    let loader = federal::parse::get_federal_data_loader_2013(&FileFinder::find_ec_data_repository());
    //let metadata = loader.read_raw_metadata("ACT")?;
    //serde_json::to_writer_pretty(std::io::stdout(),&metadata)?;
    //println!("{:#?}",metadata);

    let data = loader.load_cached_data("ACT")?;
    data.print_summary();
    let transcript = data.distribute_preferences::<FederalRulesPre2021>(&mut Randomness::ReverseDonkeyVote);
    let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
    let file = File::create("transcript.json")?;
    serde_json::to_writer_pretty(file,&transcript)?;
    //let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
    //official_transcript.compare_with_transcript(&transcript.transcript,|tally|tally as f64);
    Ok(())
}