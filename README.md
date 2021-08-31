# ConcreteSTV

ConcreteSTV is a suite of programs used to count 
[Single Transferable Vote (STV)](https://en.wikipedia.org/wiki/Single_transferable_vote) elections, 
which are a form of preferential voting used to elect multiple candidates. They are
widely used in Australian elections.

Unlike many forms of voting, the actual counting of STV elections is not trivial, and
indeed there are many plausible quite different sets of rules for STV. 
The aim of ConcreteSTV is to implement versions of STV that are actually used in
a variety of jurisdictions.

ConcreteSTV is a rewrite of an [earlier project](https://github.com/SiliconEconometrics/PublicService)
but does not yet have all the features of the earlier project. However it
is more user friendly, and future development will be concentrating on
this project.  

Results from the earlier project were used to find and fix bugs [in the ACT STV count](https://github.com/SiliconEconometrics/PublicService/blob/master/CountVotes/2020%20Errors%20In%20ACT%20Counting.pdf), and to identify bugs [in the NSW count](https://raw.githubusercontent.com/SiliconEconometrics/PublicService/master/CountVotes/2016%20NSW%20LGE%20Errors.pdf) which led the NSW Parliament to simplify the rules. Everyone is encouraged to use this code to double-check and correct election results.

## Currently Supported Elections

- **Federal** My interpretation of the Australian Federal Senate election system. The legislation
  appears ambiguous in various places to me, in particular when rules 273(17) or 273(18) are
  applied. I interpret both as after all exclusions and surplus distributions are finished. I don't
  claim this is more reasonable than other interpretations. I also have a variety of
  interpretations of the rules the Australian Electoral Commission (AEC) actually used in recent
  elections. I cannot find out what rules they _actually_ used as the source code of their
  program to count them is a tightly held secret, but one can make a good guess by looking
  at the provided distribution of preferences.
  - **AEC2013** This is my interpretation of the rules actually used by the AEC for the 2013 election. 
    It is very similar to *Federal* except
      - When resolving 3 way ties by looking at prior counts, any difference is used as a discriminator,
        instead of requiring that each has a different count. Evidence: NSW 2016, special count
        with Rod Cullerton excluded, count 49. Assumed 2013 same as 2016.
      - Rule (17) is applied after all exclusions and surplus distributions are finished (same as my interpretation). 
        Evidence: 2013 SA, count 228
      - But Rule (18) is applied after all surplus distributions, and the first transfer of an exclusion are finished. 
        Assumed same as 2016, where Qld, WSW, Vic and WA are all evidence.
  - **AEC2016** This is my interpretation of the rules actually used by the AEC for the 2016 election.
    It is very similar to *AEC2013*, except the Bulk Exclusion rules are not applied (evidence : this should crop
    up frequently)
  - **AEC2019** This is my interpretation of the rules actually used by the AEC for the 2019 election.
    It is very similar to *AEC2016*, except rule (18) is applied after determining who to exclude but
    before transferring any votes (evidence 2019 NSW, count 429)
    
More jurisdictions are expected to be added soon.

## To compile

ConcreteSTV is written in [Rust](https://www.rust-lang.org/). Install Rust (latest stable version
recommended), then run, in this directory,
```bash
cargo build --release
```

This will create several binary programs in the `target/release` directory.

## To get real election data (parse_ec_data)

Before we can count an election, we need the votes to count. ConcreteSTV uses a format 
with extension `.stv` to store votes and some metadata about the election.

Some electoral commissions publish
a list of votes that are used as the basis of their counts. Let's choose the federal 2019 election, state Tasmania.

We will use a subdirectory of the main project for these examples.
```bash
mkdir work
cd work
```
The example commands are for Linux; Windows and MacOS will be very similar.

We need a **.stv** file containing a list of votes and some metadata. We can get this from
the program `parse_ec_data` via the command
```bash
../target/release/parse_ec_data AEC2019 TAS --out TAS2019.stv
```
This says parse data for the 2019 federal election, state TAS, and put the results into 
the file `TAS2019.stv`. Running it produces an error as we don't actually have the
two needed election files to parse, but it tells us where to get them (or at least one of them):
```text
Error: Missing file SenateFirstPrefsByStateByVoteTypeDownload-24310.csv look in https://results.aec.gov.au/24310/Website/SenateDownloadsMenu-24310-Csv.htm
```
Go to said URL (or [use this direct link](https://results.aec.gov.au/24310/Website/External/SenateStateFirstPrefsByPollingPlaceDownload-24310-TAS.zip)), download 'First preferences by state by vote type (CSV)' into your current directory, then try again. 
```text
Error: Missing file aec-senate-formalpreferences-24310-TAS.zip look in https://results.aec.gov.au/24310/Website/SenateDownloadsMenu-24310-Csv.htm
```
Sorry, we need another file. Download 'Formal Preferences - Tasmania' from the website ([or this direct link](https://results.aec.gov.au/24310/Website/External/aec-senate-formalpreferences-24310-TAS.zip)), then try again
```bash
../target/release/parse_ec_data AEC2019 TAS --out TAS2019.stv
```
It will parse for a second or two, and produce the desired file. Check in your directory, 
there should be a roughly 11MB file `TAS2019.stv`. You may look at it with a JSON viewer
if you wish.

## To count (concrete_stv)

The `concrete_stv` program takes in a rule specification and a .stv file, and produces
a *.transcript* file containing the distribution of preferences for each count. We will
use the AEC2019 rules for this.

```bash
../target/release/concrete_stv AEC2019 TAS2019.stv
```

This will pause for a second as it reads the input, then print out a text version of the
distribution of preferences, which is somewhat hard to read. 
It will also have created a roughly hundred kilobyte JSON file `TAS2019_AEC2019.transcript`, which we
will use in the next section for a prettier view. 

Note that you can pass --help as an option to either of these programs for details on options.

## To view a transcript

The `Viewer` folder of this project contains a web based viewer for transcript files.
Open `Viewer/Viewer.html` in a web browser. 

In the upper left corner, there will be a *Browse* button. Use it to select the `TAS2019_AEC2019.transcript`
file from before.

This will produce a large image similar to this:

![Web browser view of transcript of 2019 EC election](readme_images/Tas2019Transcript.png)

Elected candidates have a background of pale green, excluded candidates of pale red. 
Votes, and the differentials for each count, are listed by default; you can also see 
the number of papers by selecting the "Show papers" box.

You can compare this to the [AEC provided transcript](https://results.aec.gov.au/24310/Website/External/SenateStateDop-24310-TAS.pdf).

Other STV counting programs include Grahame Bowland's [Dividebatur](https://github.com/grahame/dividebatur) 
and its successor [Dividebatur2](https://github.com/grahame/dividebatur2), Lee Yingtong Li's [OpenTally](https://yingtongli.me/git/OpenTally/), and Milad Ghale's [formally verified STV](https://github.com/MiladKetabGhale/STV-Counting-ProtocolVerification).

## Example data files

The `examples` directory contains some interesting contrived examples where the rules used matter.
```bash
cd ../examples
../target/release/concrete_stv AEC2013 MultipleExclusionOrdering.stv 
```
has winners W1, W2, W7, W6, W5, W4.
```bash
../target/release/concrete_stv AEC2016 MultipleExclusionOrdering.stv 
```
has different winners: W1, W3, W4, W5, W6, W7. This demonstrates that bulk exclusion can affect who wins, on a contrived example.
```bash
../target/release/concrete_stv AEC2019 MultipleExclusionOrdering.stv 
```
has winners W1, W3, W7, W6, W5, W4, the same as 2016 but in a different order. This shows that the priority of rule (18) affects the order in which candidates are considered elected.

## File formats

Both the .stv and .transcript files are JSON format. 

The .stv files are a straight forward JSON representation of the `ElectionData` structure defined in
[election_data.rs](stv/src/election_data.rs) which reference structures in [ballot_paper.rs](stv/src/ballot_paper.rs), 
and metadata given by the `ElectionMetadata` structure in [ballot_metadata.rs](stv/src/ballot_metadata.rs)

The .transcript files are a straight forward JSON representation of the `TranscriptWithMetadata`
structure defined in [distribution_of_preferences_transcript.rs](stv/src/distribution_of_preferences_transcript.rs),
which uses the same metadata format.

## LaTeX tables

You can convert a .stv or (more commonly) .transcript file to a LaTeX table
by the `transcript_to_latex` program:
```bash
../target/release/transcript_to_latex --deltas MultipleExclusionOrdering_AEC2016.transcript > MultipleExclusionOrdering_AEC2016.tex
```

Note that these tables are generally too large to fit onto a normal page. To restrict the
table to a small number of candidates, use the `--candidates` option. Use the `--help`
option for details.

## Copyright

This program is Copyright 2021 Andrew Conway.

This file is part of ConcreteSTV.

ConcreteSTV is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

ConcreteSTV is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

Or course any files you download from electoral commissions (or elsewhere)
are likely covered by their own licenses.

## Contact

Contact the author andrew at andrewconway.org