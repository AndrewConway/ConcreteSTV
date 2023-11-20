# How to run casual vacancies for the ACT

In the ACT Legislative Assembly a casual vacancy is resolved by extracting the votes that were used
to elect the former MLA, and the effectively doing instant runoff voting on the extracted votes, which may
have different transfer values associated with them. There are some [IMO generally minor ambiguities](CasualVacanciesAmbiguities.md) in
the legislation.

An example will be provided for the 2021 casual vacancy for ex-MLA Alistair Coe.

## Compile ConcreteSTV

compile ConcreteSTV as in [the main README.md](../README.md)
```shell
cd ConcreteSTV
cargo build --release
```

## Getting the data for the Yerrabi 2020 election

Alistair Coe was elected in the Yerrabi 2020 election, so first we need to get the
vote data for this. This comes from the [Elections ACT website](https://www.elections.act.gov.au/elections_and_voting/2020_legislative_assembly_election/ballot-paper-preference-data-2020-election). 
Of course note the copyright restrictions at [https://www.elections.act.gov.au/copyright](https://www.elections.act.gov.au/copyright).

```shell
mkdir act_casual_vacancy
cd act_casual_vacancy
wget https://www.elections.act.gov.au/__data/assets/text_file/0007/1662064/Electorates.txt
wget https://www.elections.act.gov.au/__data/assets/text_file/0010/1662067/Groups.txt
wget https://www.elections.act.gov.au/__data/assets/text_file/0006/1662063/Candidates.txt
wget https://www.elections.act.gov.au/__data/assets/text_file/0009/1662084/YerrabiTotal.txt
```

Now parse them and produce a .stv file via a command like the following (changing the path to the executable depending on what directory you are in.)
```bash
../target/release/parse_ec_data ACT2020 Yerrabi --out Yerrabi2020.stv
```

This should create a file of size ~1.2MB called `Yerrabi2020.stv`

## Run the election and extract the votes associated with Alistair Coe's election

Note that Alistair COE is candidate number 6, where we are counting the first candidate listed as number zero.

Now run the election, telling it to extract the votes used to elect candidate number 6 into the file `VotesToElectAlistairCoe.stv`.
As before, you may have to change the path if you are using a different directory.
```bash
../target/release/concrete_stv ACT2021 Yerrabi2020.stv --extract "UsedToElectACT:6;file:VotesToElectAlistairCoe.stv"
```

This should create two files:
 * `Yerrabi2020_ACT2021.transcript` giving the transcript of the election. View this 
    in a web browser with [docs/Viewer.html](../docs/Viewer.html)
    or the version [hosted on github](https://andrewconway.github.io/ConcreteSTV/Viewer.html)).
 * `VotesToElectAlistairCoe.stv` giving the votes (and their associated transfer values) used
    to elect Alistair Coe.

## Run the casual vacancy election.

Note that the rules for the casual vacancy election are almost identical to the rules for the general election.
The `ACT2021` rules will generally do a perfect job of matching the rules other than the recomputation of
the quota each count. This will not change who is elected or the tallies at any count, 
but it may occasionally cause the counts to continue unnecessarily far (it usually won't and doesn't in this specific case).

We also have to say which candidates were eligible and chose to stand. Do this by excluding the other
candidates. Their numbers are listed in the command below (count them in the original list of candidates, starting
from zero).

As before, you may have to change the path if you are using a different directory.
```bash
../target/release/concrete_stv ACT2021 VotesToElectAlistairCoe.stv --exclude 0,5,6,7,9,10,11,15,16,18,19,20,21,22
```

This will create a transcript file `VotesToElectAlistairCoe_ACT2021.transcript` of the casual vacancy scrutiny sheet;
view this
in a web browser with [docs/Viewer.html](../docs/Viewer.html)
or the version [hosted on github](https://andrewconway.github.io/ConcreteSTV/Viewer.html)). 
Note that Jame Milligan wins with a tally of 5198.487179. Compare this to the 
[official scrutiny sheet](https://www.elections.act.gov.au/__data/assets/pdf_file/0010/1731178/Table-2-Alistair-Coe.pdf).

# Changes to run the 2022 Giulia Jones casual vacancy.

This is similar except it is in Murrumbidgee. Assuming you have already downloaded the files above, you will
also need to download Murrumbidgee data:
(remembering the copyright restrictions at [https://www.elections.act.gov.au/copyright](https://www.elections.act.gov.au/copyright).)

```shell
wget https://www.elections.act.gov.au/__data/assets/text_file/0009/1662075/MurrumbidgeeTotal.txt
../target/release/parse_ec_data ACT2020 Murrumbidgee --out Murrumbidgee2020.stv
```

and run the data extracting the votes for Giulia Jones (candidate number 20 starting from 0).

```bash
../target/release/concrete_stv ACT2021 Murrumbidgee2020.stv --extract "UsedToElectACT:20;file:VotesToElectGiuliaJones.stv"
```

Now run with the appropriate list of excluded candidates.

```bash
../target/release/concrete_stv ACT2021 VotesToElectGiuliaJones.stv --exclude 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,20,23,25,28
```

Now look at `VotesToElectGiuliaJones_ACT2021.transcript` in the viewer. 

Note that the [official count transcript](https://www.elections.act.gov.au/__data/assets/pdf_file/0016/2022082/Scrutiny-Sheet-Table-2.pdf)
matches the ConcreteSTV counts perfectly up to count 13, at which ConcreteSTV declares Ed Cocks the winner
on 4385.774235 votes which is well more than half the total number of votes for continuing candidates, and, 
according to my (non-lawyer) reading of the legislation
section 4.3 15(2), "the scrutiny shall cease". The official transcript continues unnecessarily for a few more counts. This is
harmless as it cannot change who wins.

# Changes to run the 2023 Johnathan Davis casual vacancy.

This is similar except it is in Brindabella. Assuming you have already downloaded the files above, you will
also need to download Brindabella data:
(remembering the copyright restrictions at [https://www.elections.act.gov.au/copyright](https://www.elections.act.gov.au/copyright).)

```shell
wget https://www.elections.act.gov.au/__data/assets/text_file/0005/1662062/BrindabellaTotal.txt
../target/release/parse_ec_data ACT2020 Brindabella --out Brindabella2020.stv
```

and run the data extracting the votes for Johnathan Davis (candidate number 7 starting from 0).

```bash
../target/release/concrete_stv ACT2021 Brindabella2020.stv --extract "UsedToElectACT:7;file:VotesToElectJohnathanDavis.stv"
```

Running the casual vacancy requires knowing who is standing, which is not decided at the time of writing.
So I cannot say who to exclude.
At the minimum one will have to exclude the current MLAs. The command below assumes everyone else is standing:

```bash
../target/release/concrete_stv ACT2021 VotesToElectJohnathanDavis.stv --exclude 2,3,7,10,14
```

Now look at `VotesToElectJohnathanDavis_ACT2021.transcript` in the viewer.