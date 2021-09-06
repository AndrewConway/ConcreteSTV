# ConcreteSTV

ConcreteSTV is a suite of programs used to count 
[Single Transferable Vote (STV)](https://en.wikipedia.org/wiki/Single_transferable_vote) elections, 
which are a form of preferential voting used to elect multiple candidates. They are
widely used in Australian elections.

Unlike many forms of voting, the actual counting of STV elections is not trivial, and
indeed there are many plausible quite different sets of rules for STV. 
The aim of ConcreteSTV is to implement versions of STV that are actually used in
a variety of jurisdictions. This emphasis on perfectly matching actual the
algorithms used in actual, concrete elections is where the name comes from.

ConcreteSTV is a rewrite of an [earlier project](https://github.com/SiliconEconometrics/PublicService)
but does not yet have all the features of the earlier project. However it
is more user friendly, and future development will be concentrating on
this project.  

Results from the earlier project were used to find and fix bugs [in the 2020 ACT STV count](reports/2020%20Errors%20In%20ACT%20Counting.pdf), and to identify bugs in the [2012](reports/NSWLGE2012CountErrorTechReport.pdf) and [2016](reports/2016%20NSW%20LGE%20Errors.pdf) NSW count which led the NSW Parliament to simplify the rules. Everyone is encouraged to use this code to double-check and correct election results.

## Currently Supported Election Rules

- **Federal** My interpretation of the Australian Federal Senate election system. The federal rules have
  surplus distributed among all ballots, continuing or not, with one shared transfer value. The legislation changed
  significantly between 2013 and 2016, removing formal party tickets and changing formality
  requirements, but those get dealt with in this system before these count rules apply. The legislation
  appears ambiguous in various places to me, in particular when rules 273(17) or 273(18) are
  applied. I interpret both as after all exclusions and surplus distributions are finished. I don't
  claim this is more reasonable than other interpretations. I also have a variety of
  interpretations of the rules the Australian Electoral Commission (AEC) actually used in recent
  elections. I cannot find out what rules they _actually_ used as the source code of their
  program to count them is a tightly held secret, but one can make a good guess by looking
  at the provided distribution of preferences. See [Our Report](reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf) for
  details of the differences between the below specific rules.
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
    
- The ACT Legislative Assembly is elected by STV with a generally well written and minimal
  set of rules, with surplus distributed amongst continuing ballots in the last parcel.
  A rule for restricting the resulting transfer value from exceeding the transfer value in the
  last parcel can lead to votes effectively set aside; such votes are counted by ElectionsACT
  as lost to rounding, which is harmless other than being mildly confusing. I have emulated
  this behaviour.
  In 2000 the legislation changed to count votes to 6 decimal digits instead of 
  as integers. This introduced a (probably unintended) problem in the legislation as a surplus 
  was constrained to having at least 1 vote above quota. This made sense and was equivalent to
  greater than zero when counts were all integers, but was unsatisfying with fractional votes - what
  should one do with a candidate who got 0.5 votes above a quota? ElectionsACT investigated this
  question in depth and concluded (sensibly IMHO) that anything above zero was actually intended to be a surplus. 
  This seems consistent with the spirit and implied intention if not the literal wording of the legislation, so I have
  adopted the same behaviour. They did however also introduce a variety of new [bugs](reports/2020%20Errors%20In%20ACT%20Counting.pdf) 
  at the same time which we pointed out. In March 2021 ElectionsACT quietly changed the distribution of preferences on their
  website having fixed the bugs we reported. This leads to different rules needed for 2020 and 2021.
  - **ACTPre2020** : This is my interpretation of the rules used by ElectionsACT for the 2008, 2012, and 2016
  elections. It seems to match the legislation well.
  - **ACT2020** : This is my interpretation of the buggy set of rules used by ElectionsACT for the 2020 election.
    Use this ruleset to match the [now removed](https://web.archive.org/web/20201127025950/https://www.elections.act.gov.au/elections_and_voting/2020_legislative_assembly_election/distribution-of-preferences-2020) original 2020 election results.
    It differs from ACT2021 by emulating the following bugs:
    * Round votes to nearest instead of down. (Generally small effect, but it allows negative votes to be lost to rounding, and thus for more than the allowed number of candidates to achieve a quota. Acknowledged by ElectionsACT and fixed in 2021.)
    * Round transfer values to six digits if rule 1C(4) applies. (Like previous, except larger effect. Acknowledged by ElectionsACT and fixed in 2021)
    * Count transfer values computed in rule 1C(4) as having a different value to all other transfer values with the same value. (Big effect, as it can change which vote batch is the last parcel. [Denied](https://www.elections.act.gov.au/__data/assets/pdf_file/0011/1696160/Letter-to-V-Teague-30-Nov-2020_Redacted.pdf) by ElectionsACT but still fixed in 2021.)
    * Round exhausted votes to an integer when doing exclusions (instead of 6 decimal places). This can't change who is elected, just the transcript.
    * Surplus distribution is completed even after everyone is elected. This can't change who is elected, just the transcript.
  - **ACT2021** : This is my interpretation of the fixed set of rules used by ElectionsACT to recount the 2020 election in 2021.
    It differs from ACTPre2020 in counting votes to 6 decimal places. To match the results currently (as of March 2021) on the 
    [ElectionsACT website](https://www.elections.act.gov.au/elections_and_voting/2020_legislative_assembly_election/distribution-of-preferences-2020)
    use ACT2021 ruleset rather than ACT2020.
    

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
Go to said URL (or [use this direct link](https://results.aec.gov.au/24310/Website/External/SenateFirstPrefsByStateByVoteTypeDownload-24310.csv)), download 'First preferences by state by vote type (CSV)' into your current directory, then try again. 
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

### Election data formats understood

Currently parse_ec_data can accept (as first argument) the following elections:
* Federal Senate : AEC2013, AEC2016, AEC2019 [AEC](https://results.aec.gov.au/)
* ACT Legislative assembly : ACT2008, ACT2012, ACT2016, ACT2020 [ElectionsACT](https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections)

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

One complication is the representation of numbers. In .stv files, there are only integers and they are represented by numbers.
In .transcript files there are non-integers. These are represented in the program as ratios of integers or as scaled integers, for exact precision.
Writing them as JSON numbers could lead to loss of precision.
* Transfer values are stored as JSON strings, either "1" or a ratio like "34/234".
* Most vote counts and ballot paper counts are stored as JSON numbers (integers).
* When a vote count could be a non-integer (e.g. ACT2020 or ACT2021 rules), vote tallys are stored as strings like "345.288272". Ballot counts are still stored as JSON numbers as they are integers
* Votes lost to rounding is a special case, as unlike all other numbers mentioned here it can be negative (ACT2020 rules). These are stored as JSON strings, even when using rule sets where they must be integers.

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