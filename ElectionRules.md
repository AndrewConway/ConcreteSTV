# Election Rules supported by ConcreteSTV

This document described in details the rules options for ConcreteSTV. Allowed options are presented in bold
in the text below.

## Federal Senate

The federal rules have
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

The rules actually used in each year are a secret and the AEC has persecuted those
asking for the source code of the counting algorithm under FOI, so the actual rules are my best guess.

- **FederalPre2021** My interpretation of the Australian Federal Senate election system prior to the
  2021 legislation changes. Used to be called **Federal** but renamed as the legislation changed.
- **FederalPost2021** My interpretation of the Australian Federal Senate election system post the
  2021 legislation changes. Changes are to tie resolution and the removal of bulk exclusion, making it essentially **AEC2016**.
- **FederalPost2021Manual** My interpretation of the Australian Federal Senate election system for manual counting post the
  2021 legislation changes. Same as **FederalPost2021** except bulk exclusion is still included, making it essentially **AEC2013**.
- **AEC2013** This is my interpretation of the rules actually used by the AEC for the 2013 and 2014 election.
  It is very similar to *FederalPre2021* except when resolving 3 way ties by looking at prior counts, any difference is used as a discriminator,
          instead of requiring that each has a different count. Evidence: Vic 2013, count 60. I assume 2014 is the same in this respect as 2013 and 2016, where there is a similar case.
- **AEC2016** This is my interpretation of the rules actually used by the AEC for the 2016 election.
  It is very similar to *AEC2013*, except the Bulk Exclusion rules are not applied (evidence : this crops
  up frequently).
- **AEC2019** This is my interpretation of the rules actually used by the AEC for the 2019 and 2022 election.
  It is very similar to *AEC2016* or *FederalPost2021*, except rule (18) is applied after determining who to exclude but
  before transferring any votes (evidence 2019 NSW, count 429 and 2022 QLD count 266).
  
Update: A [personal communication (reproduced with permission)](reports/RecommendedAmendmentsSenateCountingAndScrutinyResponse/18_10_2021_-_Dr_Andrew_Conway_and_Prof._Vanessa_teague_-_Senate_Counting_and_Scrutiny.pdf) 
from the Electoral Commissioner clarified some of these interpretations. Note that the personal response
incorrectly claimed that our report "validates the accuracy of the Senate election outcomes", which is the 
opposite of our view, and "this is a more appropriate interpretation as acknowledged in your paper". I don't
know exactly what that was intended to mean, but we certainly don't think that what they did was an appropriate interpretation
of the legislation. I will try not to put words into his mouth in this interpretation of said letter.
He stated something difficult to interpret about the tie elimination, which I believe confirms that
they do it the way we believed, rather than the way specified by the legislation. 
He also verified our interpretation of the bulk exclusion rules (that they are compulsory),
and that rule (18) should not cut short an exclusion. He did not comment on the fact that that was not
what the AEC did for either in 2019, or for the bulk exclusion in 2016. We interpret his interpretation
of rule (17) to be our interpretation for surplus distributions, but he did not mention the (very likely to
change an election outcome) priority of exclusions with respect to rule (17). He did not comment on whether
any of these would be fixed for the next election, or whether they would adopt any of our recommendations. We are
seeking clarification. 

Update 2: The resolution has largely been to change the legislation (Dec 2021) to match what the AEC did.
The tie breaking for exclusions has been changed to what the AEC did (which is arguably better than what the legislation
had required), and the horrible Bulk Exclusion rules no longer apply when the count is done by computer
(but do apply when the count is done by hand, which could produce different results). The error in
the AEC's handling of rule 18 is not addressed in the legislation. The legislation does include a 
clause requiring some checking of the listed votes against the paper ballots, which is a great thing
as it will encourage the AEC to actually provide some meaningful evidence that the election result is correct.
A more detailed discussion is [available](federal/legislation/AssuranceOfSenateCountingAct2021.md).
This has cause the creation of ruleset **FederalPost2021** and **FederalPost2021Manual**.

## ACT Legislative Assembly

The ACT Legislative Assembly is elected by STV with a generally well written and minimal
set of rules, with surplus distributed amongst continuing ballots in the last parcel.
A rule for restricting the resulting transfer value from exceeding the transfer value in the
last parcel can lead to votes effectively set aside; such votes are counted by ElectionsACT
as lost to rounding, which is harmless other than being mildly confusing. I have emulated
this behaviour.
In 2020 the legislation changed to count votes to 6 decimal digits instead of
as integers. This introduced a (probably unintended) problem in the legislation as a surplus
was constrained to having at least 1 vote above quota. This made sense and was equivalent to
greater than zero when counts were all integers, but was unsatisfying with fractional votes - what
should one do with a candidate who got 0.5 votes above a quota? ElectionsACT investigated this
question in depth and concluded (sensibly IMHO) that anything above zero was actually intended to be a surplus.
This seems consistent with the spirit and implied intention if not the literal wording of the legislation, so I have
adopted the same behaviour. They did however also introduce a variety of new [bugs](reports/2020%20Errors%20In%20ACT%20Counting.pdf)
at the same time which we pointed out. In March 2021 ElectionsACT quietly changed the distribution of preferences on their
website having fixed the bugs we reported. This leads to different rules needed for 2020 and 2021.

Prior to 2020, ElectionsACT made their counting code publicly available (and it was unusually good quality).
In 2020 they made it a secret (just in time for the bugs).

- **ACTPre2020** : This is my interpretation of the rules used by ElectionsACT for the 2008, 2012, and 2016
  elections. It seems to match the legislation well.
- **ACT2020** : This is my interpretation of the buggy set of rules used by ElectionsACT for the 2020 election.
  Use this ruleset to match the [now removed](https://web.archive.org/web/20201127025950/https://www.elections.act.gov.au/elections_and_voting/2020_legislative_assembly_election/distribution-of-preferences-2020) original 2020 election results.
  It differs from ACT2021 by emulating the following bugs:
  * Round votes to nearest instead of down. (Generally small effect, but it allows negative votes to be lost to rounding, and thus for more than the allowed number of candidates to achieve a quota. Acknowledged by ElectionsACT and fixed in 2021.)
  * Round transfer values to six digits if rule 1C(4) applies. (Like previous, except larger effect. Acknowledged by ElectionsACT and fixed in 2021)
  * Count transfer values computed in rule 1C(4) as having a different value to all other transfer values with the same value. (Big effect, as it can change which vote batch is the last parcel. 
    [Denied](https://www.elections.act.gov.au/__data/assets/pdf_file/0011/1696160/Letter-to-V-Teague-30-Nov-2020_Redacted.pdf) [publicly](https://www.hansard.act.gov.au/hansard/2021/comms/jacs01a.pdf) by ElectionsACT but still fixed in 2021.)
  * Round exhausted votes to an integer when doing exclusions (instead of 6 decimal places). This can't change who is elected, just the transcript.
  * Surplus distribution is completed even after everyone is elected. This can't change who is elected, just the transcript.
- **ACT2021** : This is my interpretation of the fixed set of rules used by ElectionsACT to recount the 2020 election in 2021.
  It differs from ACTPre2020 in counting votes to 6 decimal places. To match the results currently (as of March 2021) on the
  [ElectionsACT website](https://www.elections.act.gov.au/elections_and_voting/2020_legislative_assembly_election/distribution-of-preferences-2020)
  use ACT2021 ruleset rather than ACT2020.

## NSW Local Government (2017 and earlier)

The NSW local government elections use somewhat random selection for surplus distribution. This means
that rerunning the count with different random choices can significantly change the outcome (often changing
who is elected).
The NSWEC does not provide their choices, so attempting to reproduce their exact outcomes is not practical.

The legislation is very ambiguous, and my implementation is instead based on a specification the NSWEC
produces, _Functional Requirements for Count Module_. 

- **NSWECRandomLGE2012** My interpretation of the rules used by the NSWEC for the 2012 local government election. Same as NSWECRandomLGE2016 except it sometimes computes the last parcel incorrectly (see [our report](reports/NSWLGE2012CountErrorTechReport.pdf))
- **NSWECRandomLGE2016** My interpretation of the rules used by the NSWEC for the 2016 local government election. Same as NSWECRandomLGE2017 except it sometimes gets tie resolutions for exclusions and fractions incorrect (see [our report](reports/2016%20NSW%20LGE%20Errors.pdf))
- **NSWECRandomLGE2017** My interpretation of the rules used by the NSWEC for the 2017 local government election. 

## NSW Local Government (2021 and later)

The NSW local government election count algorithm changed prior to the 2021 election as the old legislation was probabilistic, and frequently
gave different results when counted multiple times.

The [new legislation](nsw/NSWLocalCouncilLegislation2021.md) is very ambiguous, and while I have
made an [interpretation](nsw/NSWLocalCouncilLegislation2021Commentary.md) of it, I would not say
that my interpretation precludes just as valid other interpretations which are likely to result
in different candidates being elected. After writing my interpretation, 
the NSWEC published the transcript of distribution of preferences, from which I was able
to work out my interpretation of their interpretation, which is now also in the above link.

Note that rule (7)(4)(a), `... The surplus fraction is equal to the resulting fraction or (if the fraction exceeds 1) to 1,`
is unambiguous but is not reasonable, as the _resulting fraction_ may be negative, which causes all sorts of problems.
This situation did not happen to come up in the 2021 election, so I cannot say whether the NSWEC chose to
obey the literal legislation or to set the surplus fraction to 1 if the resulting fraction were negative.
For this reason I have make rules that cover both cases.

- **NSWLocalGov2021** My interpretation, for what it is worth
- **NSWECLocalGov2021** My interpretation of the rules used by the NSWEC for the 2021 NSW local government elections, assuming they didn't take (7)(4)(a) literally
- **NSWECLocalGov2021Literal** My interpretation of the rules used by the NSWEC for the 2021 NSW local government elections, assuming they did take (7)(4)(a) literally. This can result in candidates getting negative tallies or more candidates than there are vacancies going over quota.

## NSW Legislative council

The NSW legislative council elections use somewhat random selection for surplus distribution, very similar to the old
NSW local government legislation. This means that rerunning the count with different random choices can significantly change the transcript. 
The NSWEC does not provide their choices, so attempting to reproduce their exact outcomes is not practical.

The legislation is very ambiguous, and my implementation is instead based on a specification the NSWEC
produces, _Functional Requirements for Count Module_.

- **NSWECRandomLC2015** My interpretation of the rules used by the NSWEC for the 2015 local government election. Same as NSWECRandomLC2019 except it sometimes computes the last parcel incorrectly (see [our report](reports/NSWLGE2012CountErrorTechReport.pdf). 
  The situation where this error occurs does not crop up in the official count, so it is not clear whether this bug was actually present in the program used by the NSWEC, but the bug was present in the pseudocode for the documentation for the specification of the program at the time, so my best guess is that it was.)
- **NSWECRandomLC2019** My interpretation of the rules used by the NSWEC for the 2019 and 2023 legislative council election. 

## Victorian upper house (Legislative Council)

The Victorian rules in the Electoral Act 2022 are relatively well written and mostly unambiguous, 
although 114A(28)(c) conflicted with 114A(12)(b). The VEC seems to have (reasonably)
come down on the side of 114A(12)(b), and indeed the legislation changed in 2018 
changing 114A(28)(c), fixing the conflict and confirming the VEC's interpretation.

Unfortunately the VEC does not publish full vote data, and so it is difficult to
verify their count. The 2014 data matches the output of the *Vic2018*
rule set for the regions not updated in 2015.

- **Vic2018** My interpretation of the rules post the 2018 conflict resolution
              (and a reasonable if not literal interpretation of prior rules).

## WA upper house (Legislative Council)

The WA rules are relatively straight forward, although there is some ambiguity about
when the election ends, and the division of transfers during a surplus is very
ambiguous. See the [source code](wa/src/lib.rs) for more detailed comments.

Thanks to Yington Li for some conversation about this ambiguity; errors
are my own.

Unfortunately the WAEC does not publish full vote data, and so it is difficult to
verify their count. Even worse, while their website is quite user friendly, most years
they only publish a summary of the distribution of preferences that is insufficient for
me to work out what they did. However the 2008 distribution of preferences is 
sufficiently detailed to be able to check most of the ambiguities.

- **WA2008** My interpretation of the Western Australian Legislative Council rules consistent with the 2008 published official distribution of preferences.  



