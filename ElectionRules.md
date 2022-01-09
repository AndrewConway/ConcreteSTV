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

- **Federal** My interpretation of the Australian Federal Senate election system.
- **AEC2013** This is my interpretation of the rules actually used by the AEC for the 2013 election.
  It is very similar to *Federal* except
  - When resolving 3 way ties by looking at prior counts, any difference is used as a discriminator,
          instead of requiring that each has a different count. Evidence: NSW 2016, special count
          with Rod Cullerton excluded, count 49. I assume 2013 same as 2016.
  - Rule (17) is applied after all exclusions and surplus distributions are finished (same as my interpretation).
    This can be seen in 2013 SA, count 228. However I believe Rule (18) is applied after all surplus distributions, and the first transfer 
    of an exclusion are finished. This is assumed  same as 2016, where Qld, WSW, Vic and WA are all evidence.
- **AEC2016** This is my interpretation of the rules actually used by the AEC for the 2016 election.
  It is very similar to *AEC2013*, except the Bulk Exclusion rules are not applied (evidence : this crops
  up frequently)
- **AEC2019** This is my interpretation of the rules actually used by the AEC for the 2019 election.
  It is very similar to *AEC2016*, except rule (18) is applied after determining who to exclude but
  before transferring any votes (evidence 2019 NSW, count 429)
  
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
The tie breaking has been changed to what the AEC did (which is arguably better than what the legislation
had required), and the horrible Bulk Exclusion rules no longer apply when the count is done by computer
(but do apply when the count is done by hand, which could produce different results). The error in
the AEC's handling of rule 18 is not addressed in the legislation. The legislation does include a 
clause requiring some checking of the listed votes against the paper ballots, which is a great thing
as it will encourage the AEC to actually provide some meaningful evidence that the election result is correct.
I will create new rulesets taking the new legislation into account very soon.

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
  
## NSW Local Government

The NSW local government elections used to use similar legislation to the NSW legislative council,
but drastically changed prior to the 2021 election as the old legislation was probabilistic, and frequently
gave different results when counted multiple times.

The [new legislation](nsw/NSWLocalCouncilLegislation2021.md) is very ambiguous, and while I have
made an [interpretation](nsw/NSWLocalCouncilLegislation2021Commentary.md) of it, I would not say
that my interpretation precludes just as valid other interpretations which are likely to result
in different candidates being elected. After writing my interpretation, 
the NSWEC published the transcript of distribution of preferences, from which I was able
to work out my interpretation of their interpretation, which is now also in the above link

- **NSWLocalGov2021** My interpretation, for what it is worth
- **NSWECLocalGov2021** My interpretation of the rules used by the NSWEC for the 2021 NSW local government elections.

