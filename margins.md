# Margins

The margin is the minimum number of votes that need to be changed in order to change the
election outcome. Computing this for an STV election in a reasonable amount of time is, in general, 
an unsolved problem. However, it is straight forward to produce an *upper bound* on the margin
by finding a specific set of ballots that can be changed to change the outcome.

ConcreteSTV provides a tool, `change_outcomes` to search for such manipulations. It tries a large
number of different manipulations, and chooses the best. The definition of *best* is not
entirely clear - for instance, two manipulations may both result in changes in who is elected,
but they may be different outcomes in which case both are of interest.

One of the purposes of margin computation is to evaluate whether it was possible for 
the election outcome to have been changed by a software bug or a
malicious actor (hacker or insider) who had a limited ability to alter votes.
For instance, it may be possible for the actor to only tamper with internet votes,
or preferences other than first preferences, or only below the line votes. In order to
support this, manipulations with different values of these properties are also reported separately.

To run, execute the program `change_outcomes` in a very similar manner to `concrete_stv`,
except a .vchange file is the output instead of a .transcript file. Run `change_outcomes --help`
for a full list of options.

Note that in this document the path to the executable is not provided, as this depends upon
what directory you run it from and whether it is in your path. Like other programs, it 
will be found in `target/release/change_outcomes` after compiling with `cargo build --release`
``

## Example of use.

Suppose we generated the `Albury.stv` file as described [here](nsw/parse_ec_data_lge.md), and
we wished to see what modifications could be produced using only iVote votes. This could be computed
using the command:
```bash
change_outcomes --unverifiable iVote --allow-verifiable false --verbose NSWECLocalGov2021 Albury.stv 
```
This will probably take several seconds to a minute to run. Remove the `--verbose` flag to not print out diagnostic messages.

This will produce the file `Albury_NSWECLocalGov2021.vchange`.

## What do you do with a .vchange file?

A `.vchange` file is a JSON file containing the original .stv file, and a list of the best
changes it found. 

### Viewing a .vchange file

This can be viewed with the same viewer used to view `.transcript` files, in the ConcreteSTV
directory at `docs/Viewer.html`. Open this fine in a web browser, and click on the button labeled
`Browse...` and select the `Albury_NSWECLocalGov2021.vchange` file created above. This will create
a table like

![Web browser extract for Albury](readme_images/AlburyChangesScreenshot.png)

Note that there is some randomness in the searching algorithm, 
and different runs will sometimes produce slightly different results.

This means that there were 3 interesting changes found. The first affected 18 ballots. Its effect was for
the formerly elected candidate THURLEY David to lose a set and HAMILTON Ross to gain a set. This involved
changing some votes, at least one of which was above the line, but no first preferences were changed. 
The details section lists what the changes
were. In this case there were 18 ballots (worth 18 votes, with transfer value 1 at roughly the time
it made a difference) in which the vote
for THURLEY David (or his party for ATL votes) was changed to VAN DE VEN Henk (or his party for ATL votes).

Hovering over the details section shows exactly which votes were changed; they are shown (for space reasons)
as a list of candidate or party indices.

In general, a modification may have more than one line in the details section. Each line
represents votes from and to the same candidates, with the same transfer value.

The second line shows the same effect was possible with 34 votes added to VAN DE VEN Henk.

The third line demonstrates a way that PEARSON Lindsay could be elected; this took 404 ballots.

One can conclude from this that the margin for this election is no more than 18, although it
may be less. Indeed with different options it is possible to get a 17 vote margin. This means
that a hacker or corrupt insider who was able to change 18 ballots could definitely have changed 
the outcome of the election.

### Verifying a change

Having seen the list before, one reasonable reaction is scepticism that giving votes to
VAN DE VEN Henk will cause someone else - HAMILTON Ross - to get elected. We can see what
is going on by looking at the detailed transcript.

We can generate the original transcript by the command
```bash
concrete_stv NSWECLocalGov2021 Albury.stv
```
which will produce the normal transcript file `Albury_NSWECLocalGov2021.transcript`.

We can generate the transcript file that would result from a modification in the .vchange
file by a very similar command:
```bash
concrete_stv --modification 1 NSWECLocalGov2021 Albury_NSWECLocalGov2021.vchange
```
This creates a file `Albury_NSWECLocalGov2021_1_NSWECLocalGov2021.transcript` of the modified
transcript - the `--modification 1` command specifies that we want to apply the first modification in the .vchange file. We can look at these two transcripts in the same Viewer.html by clicking on
the `Browse...` button and selecting the desired .transcript file.

Investigating the .transcript files shows that in the unmodified case, VAN DE VEN Henk
is excluded on count 49, narrowly ahead of THURLEY David. The largest beneficiary of the
redistributed votes is THURLEY David, pushing him ahead of HAMILTON Ross. HAMILTON Ross is 
still a continuing candidate, but the one with the lowest tally at the end of count 49 
when all other continuing candidates get elected under [clause 11(3)](nsw/NSWLocalCouncilLegislation2021.md).

Investigating the transcript with modifications, the extra votes for VAN DE VEN Henk were
enough to come just ahead of THURLEY David at the end of count 48, meaning that
THURLEY David is excluded at count 49. His redistributed votes are fairly evenly split
amongst the other candidates, meaning that at the end of count 49, VAN DE VEN Henk is
still lower than HAMILTON Ross, who is then elected under [clause 11(3)](nsw/NSWLocalCouncilLegislation2021.md)..

You can get explicit details on the votes that were changed by passing the `--verbose` flag
to `concrete_stv`:
```bash
concrete_stv --verbose --modification 1 NSWECLocalGov2021 Albury_NSWECLocalGov2021.vchange
```

At the start of the text produced will be a list of the actual changes, listing the preferences before and after like:
```text
Changed 1 BTL from [MACHIN Michael, MOORE John, HEATHER Esther, DOYLE Mark, THURLEY David] to [MACHIN Michael, MOORE John, HEATHER Esther, DOYLE Mark, VAN DE VEN Henk]
Changed 1 BTL from [BARBER Andrew, KING Kylie, BOWEN Steve, THURLEY David, BETTERIDGE Daryl, HAMILTON Ross, PEARSON Lindsay, GRENFELL Brian, WATKINS Sarah, BARBER Trevor, STAR C, HARTNETT Diann, DOYLE Mark, TIERNAN Jodie, THOMAS Dianne, VENESS Rhiannon, HEATHER Esther, MOORE John, MACHIN Michael, HULL Barbara, TRATZ Mathew, GRELLMAN Emily, WALLIS Lucie, HAMILTON Claire, CHAN Aimee, SMITH Taneesha, GLACHAN Alice, HOOD Peter, MAMOUNEY Stephen, PEMBERTON Louise, DUNN Jackie, SINGH Naziya, CALE Danielle, KELLAHAN Jessica, COHN Amanda, MONTE Susie, PATTINSON Jill, ISAACS Kofi, EDWARDS Ashley, DOCKSEY Graham, BAKER Stuart, MARTIN Christopher, VAN NOORDENNEN Bill, ARMSTRONG Paul, PEARCE Garry, VAN DE VEN Henk, RYAN Christopher, ALLEN Geoffrey, CAMERON Amelia, ROWLAND Marcus, CAMERON Darren] to [BARBER Andrew, KING Kylie, BOWEN Steve, VAN DE VEN Henk, BETTERIDGE Daryl, HAMILTON Ross, PEARSON Lindsay, GRENFELL Brian, WATKINS Sarah, BARBER Trevor, STAR C, HARTNETT Diann, DOYLE Mark, TIERNAN Jodie, THOMAS Dianne, VENESS Rhiannon, HEATHER Esther, MOORE John, MACHIN Michael, HULL Barbara, TRATZ Mathew, GRELLMAN Emily, WALLIS Lucie, HAMILTON Claire, CHAN Aimee, SMITH Taneesha, GLACHAN Alice, HOOD Peter, MAMOUNEY Stephen, PEMBERTON Louise, DUNN Jackie, SINGH Naziya, CALE Danielle, KELLAHAN Jessica, COHN Amanda, MONTE Susie, PATTINSON Jill, ISAACS Kofi, EDWARDS Ashley, DOCKSEY Graham, BAKER Stuart, MARTIN Christopher, VAN NOORDENNEN Bill, ARMSTRONG Paul, PEARCE Garry, RYAN Christopher, ALLEN Geoffrey, CAMERON Amelia, ROWLAND Marcus, CAMERON Darren]
Changed 1 BTL from [BARBER Andrew, KING Kylie, BOWEN Steve, THURLEY David, BETTERIDGE Daryl, HAMILTON Ross, PEARSON Lindsay, WATKINS Sarah, GRENFELL Brian, VENESS Rhiannon, BARBER Trevor, STAR C, HARTNETT Diann, TIERNAN Jodie, THOMAS Dianne, HEATHER Esther, MOORE John, HOOD Peter, MACHIN Michael, DOYLE Mark, HULL Barbara, TRATZ Mathew, GRELLMAN Emily, WALLIS Lucie, HAMILTON Claire, CHAN Aimee, SMITH Taneesha, GLACHAN Alice, DUNN Jackie, SINGH Naziya, PEMBERTON Louise, CALE Danielle, MAMOUNEY Stephen, KELLAHAN Jessica, BAKER Stuart, DOCKSEY Graham, COHN Amanda, MONTE Susie, PATTINSON Jill, ISAACS Kofi, EDWARDS Ashley, MARTIN Christopher, VAN NOORDENNEN Bill, ARMSTRONG Paul, PEARCE Garry, VAN DE VEN Henk, RYAN Christopher, ALLEN Geoffrey, CAMERON Amelia, ROWLAND Marcus, CAMERON Darren] to [BARBER Andrew, KING Kylie, BOWEN Steve, VAN DE VEN Henk, BETTERIDGE Daryl, HAMILTON Ross, PEARSON Lindsay, WATKINS Sarah, GRENFELL Brian, VENESS Rhiannon, BARBER Trevor, STAR C, HARTNETT Diann, TIERNAN Jodie, THOMAS Dianne, HEATHER Esther, MOORE John, HOOD Peter, MACHIN Michael, DOYLE Mark, HULL Barbara, TRATZ Mathew, GRELLMAN Emily, WALLIS Lucie, HAMILTON Claire, CHAN Aimee, SMITH Taneesha, GLACHAN Alice, DUNN Jackie, SINGH Naziya, PEMBERTON Louise, CALE Danielle, MAMOUNEY Stephen, KELLAHAN Jessica, BAKER Stuart, DOCKSEY Graham, COHN Amanda, MONTE Susie, PATTINSON Jill, ISAACS Kofi, EDWARDS Ashley, MARTIN Christopher, VAN NOORDENNEN Bill, ARMSTRONG Paul, PEARCE Garry, RYAN Christopher, ALLEN Geoffrey, CAMERON Amelia, ROWLAND Marcus, CAMERON Darren]
Changed 1 ATL from [E, THE GREENS, J, D, G, A, C, H, I, LABOR] to [E, THE GREENS, I, D, G, A, C, H, LABOR]
Changed 1 ATL from [E, J, I, G, H, D, C, A, LABOR, THE GREENS] to [E, I, G, H, D, C, A, LABOR, THE GREENS]
Changed 1 ATL from [E, D, THE GREENS, J, H, I, G, LABOR, C, A] to [E, D, THE GREENS, I, H, G, LABOR, C, A]
Changed 1 ATL from [E, D, J, H, THE GREENS] to [E, D, I, H, THE GREENS]
Changed 1 BTL from [THOMAS Dianne, MAMOUNEY Stephen, BAKER Stuart, THURLEY David, KING Kylie, BOWEN Steve] to [THOMAS Dianne, MAMOUNEY Stephen, BAKER Stuart, VAN DE VEN Henk, KING Kylie, BOWEN Steve]
Changed 1 BTL from [MOORE John, BAKER Stuart, BOWEN Steve, HOOD Peter, THURLEY David] to [MOORE John, BAKER Stuart, BOWEN Steve, HOOD Peter, VAN DE VEN Henk]
Changed 1 BTL from [ISAACS Kofi, TIERNAN Jodie, BOWEN Steve, PEARSON Lindsay, THURLEY David] to [ISAACS Kofi, TIERNAN Jodie, BOWEN Steve, PEARSON Lindsay, VAN DE VEN Henk]
Changed 1 BTL from [TIERNAN Jodie, BOWEN Steve, BAKER Stuart, THURLEY David, KELLAHAN Jessica] to [TIERNAN Jodie, BOWEN Steve, BAKER Stuart, VAN DE VEN Henk, KELLAHAN Jessica]
Changed 1 BTL from [ROWLAND Marcus, BOWEN Steve, KING Kylie, THURLEY David, CAMERON Darren] to [ROWLAND Marcus, BOWEN Steve, KING Kylie, VAN DE VEN Henk, CAMERON Darren]
Changed 1 BTL from [HEATHER Esther, DOYLE Mark, THURLEY David, MACHIN Michael, MOORE John] to [HEATHER Esther, DOYLE Mark, VAN DE VEN Henk, MACHIN Michael, MOORE John]
Changed 1 BTL from [HOOD Peter, MOORE John, DOYLE Mark, THURLEY David, HEATHER Esther, MACHIN Michael, VAN DE VEN Henk, PEARCE Garry, ARMSTRONG Paul, VAN NOORDENNEN Bill, MARTIN Christopher, PEARSON Lindsay] to [HOOD Peter, MOORE John, DOYLE Mark, VAN DE VEN Henk, HEATHER Esther, MACHIN Michael, PEARCE Garry, ARMSTRONG Paul, VAN NOORDENNEN Bill, MARTIN Christopher, PEARSON Lindsay]
Changed 1 BTL from [HOOD Peter, THURLEY David, VAN DE VEN Henk, GLACHAN Alice, HAMILTON Ross] to [HOOD Peter, VAN DE VEN Henk, GLACHAN Alice, HAMILTON Ross]
Changed 1 BTL from [ISAACS Kofi, EDWARDS Ashley, THURLEY David, GLACHAN Alice, HOOD Peter, HEATHER Esther] to [ISAACS Kofi, EDWARDS Ashley, VAN DE VEN Henk, GLACHAN Alice, HOOD Peter, HEATHER Esther]
Changed 1 BTL from [PEARSON Lindsay, BAKER Stuart, THURLEY David, MOORE John, DUNN Jackie] to [PEARSON Lindsay, BAKER Stuart, VAN DE VEN Henk, MOORE John, DUNN Jackie]
Changed 1 BTL from [DOYLE Mark, THURLEY David, HEATHER Esther, MOORE John, MACHIN Michael] to [DOYLE Mark, VAN DE VEN Henk, HEATHER Esther, MOORE John, MACHIN Michael]
```

