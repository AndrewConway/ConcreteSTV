# Comments on NSW Legislation for counting algorithm for 2021 election

This is commentary on the [NSWLocalCouncilLegislation2021.md] legislation used for counting the 2021 NSW local government elections.

It includes extracts from the [New South Wales Legislation website](https://legislation.nsw.gov.au/view/whole/html/2020-10-27/sl-2005-0487#sch.5) which
is licensed under a Creative Commons Attribution 4.0 International licence (CC BY 4.0).
Based on content from the New South Wales Legislation website at 3 Dec 2021. For the latest information on New South Wales Government legislation please go to https://www.legislation.nsw.gov.au.

The following is my interpretation of the (many) ambiguities in the legislation, and my opinions. I am not a lawyer. I am merely an expert in implementation of Australian STV algorithms. I do wish
the people who drafted the legislation spoke to someone who had experience in implementing such algorithms when writing it.

This was done prior to seeing how the NSW electoral commission has interpreted this legislation. 
The legislation is very ambiguous in many places, meaning in many cases I have made a fairly arbitrary choice
between two comparably plausible interpretations. It is very likely that the NSWEC has made a different
set of choices. If their interpretation results in different candidates being elected to my interpretation, 
this does not necessarily mean that one of us has to be wrong as there is no reasonable "right" interpretatation.
I write this having found multiple errors in the NSWEC's (and other EC's) prior local government counts, and want to make it
clear that if their results differ from mine this year, that is not necessarily proof that they (or ConcreteSTV for that matter) are wrong.
I am very dubious about my (or any other) interpretation.

After the NSW Electoral Commission published their results, I made a new algorithm that matched their interpretation,
which is significantly different from mine, and could produce different candidates elected (although it does not do so
in any of the 2021 elections, assuming the provided vote lists were correct). I have added to this document comments on how they
interpreted the legislation. Their interpretation is, in my opinion, plausible, given the ambiguous legislation.

My interpretation is used for the rule set `NSWLocalGov2021`; The rules `NSWECLocalGov2021` are my interpretation
of the NSWEC's implementation, and match the dopfulldetails.xlsx perfectly (although they track some things
I do not, and I track some things they do not, and some times - e.g. City of Albury, count 13, there are
ties that are broken by lot - the choice that the NSWEC made must be specified by the `--tie` parameter)

I strongly believe that the legislation should be made unambiguous.

## History

This is new legislation totally unlike the prior legislation. The prior legislation was replaced as it was terrible -
not only was it badly written, but it was probabilistic. In an earlier project we demonstrated that recounting
the same votes would frequently result in different people being elected.

This legislation is a vast improvement. Randomness is only used when there are ties that cannot be resolved by the legislation.
However it contains many ambiguities and wierdnesses that cause problems upon trying to actually use it, as described below.

### What is a transfer?

When a candidate is excluded, the votes are transferred in multiple stages in 9(2). This is an implication
that multiple transfers may occur for one exclusion. There would be no need to specify the order in 9(2)(b)
if there were not multiple transfers. Also if it were not done in multiple transfers, then 9(2)(b)(i) would not make sense as there
would not be a unique transfer value to use in 
`the total number of ballot-papers ... multiplied by the transfer value at which the votes were transferred to the excluded candidate.`

Most Australian jurisdictions have explicit language stating this and defining a
transfer (or similar word).

This matters 
 * For defining the transfer to be completed in 7(2)
 * For tie resolution in 8(4) or 9(6)
 * For rounding in 9(2)(b)(ii)
 * For continuing candidate definitions in 10(2).

Even though this is not explicitly stated, it is reasonably unambiguous that an exclusion is divided into
multiple transfers. The NSWEC appears to make the same decision (evidence - almost everywhere).

However the situation becomes more complex when we look at clause 7, where there are likely to be multiple transfer values
produced. 7(4)(c)(ii) explicitly breaks up the computation of votes into separate computations with different transfer
values, and does the rounding in a manner suggestive of multiple transfers. If multiple transfers are *not* done
at this point, then there will be a problem later on at 9(2)(b)(i) which expects a unique transfer value. Also it would
make the rounding for each transfer value gratuitous, increasing the number of votes lost to rounding for no reason.

This seems like strong evidence for a transfer of a surplus under clause 7 to be broken into multiple transfers
by transfer value.  I assume that each different transfer value corresponds to a different transfer, 
but that for each transfer value all distributions to different candidates in 7(4)(c) are the same transfer.


However, there is a major problem with this - there is no order specified for such transfers, and the order matters
for similar reasons to the order of clause 9 transfers.

So we need to either come up with an order for clause 7, or somehow fix clause 9(2)(b)(i). Neither of these
options has an obvious answer.

#### Orders for clause 7

Other jurisdictions in Australia frequently order comparable multiple transfers by saying highest transfer value first.
This is a fine thing to do, but not at all hinted at by the legislation.

Alternatively one could look at 9(2) for inspiration and divide it up even more, by which candidate they came from as well
as transfer value. This causes needless complexity and votes lost to rounding, with no gain and little justification.

#### Multiple transfer values and clause 9(2)(b)(i)

One could fix clause 9(2)(b)(i) in the case of multiple transfer values either by changing it from
"the total number of ballot-papers transferred to the excluded candidate from a particular candidate and expressing a next available preference for a particular continuing candidate are to be multiplied by the transfer value at which the votes were transferred to the excluded candidate"
to
"the total number of ballot-papers transferred to the excluded candidate from a particular candidate and expressing a next available preference for a particular continuing candidate are to have the transfer values for each vote be summed."

Alternatively, analogously to 7(4)(c), one could add in some extra gratuitous round downs.

#### Conclusion

There is no obvious interpretation of the legislation. I consider the least intrusive thing that can be done to make
this portion of the legislation functional to be to add in an order for the multiple transfer values for clause 7,
doing highest transfer value first. I do not think that other interpretations are untenable or even less reasonable.

The interpretation used by the NSWEC (evidence example: City of Albury, count 47) is
to treat each prior sub-transfer to be treated (and rounded down) separately. This could come by interpreting
the word *transfer* in 7 to mean sub-transfer (as described below) for the purpose of 7(4) but not for the
purpose of clause 7(5). This interpretation has the benefit of being similar to the explicit separation
for exclusions, but the disadvantage of introducing a gratuitously large number of votes lost to rounding.

### When do 10(2) and 10(4) apply?

10(2) says that if a candidate obtains a quota through an exclusion, then 
"the transfer is to be completed, and all the votes to which the candidate is entitled from the transfer are to be transferred to the candidate."

Assuming (as discussed before) that there are multiple transfers in order to effect the exclusion, it is not clear
whether "the transfer" means a single one of the possibly multiple transfers involved in a single exclusion (let's call it a sub-transfer), or
if it means the entire transfer of all the votes of the candidate being excluded (lets call it a compound-transfer).

Similar rules and language are used in other jurisdictions to refer to a sub-transfer, although they generally have
an explicit definition of a transfer. This is possibly weak evidence for it meaning a sub-transfer.

Tie breaking rules refer to "preceding counts or transfers", which is evidence that "transfer" is supposed to mean
something different from "count", with the natural interpretation being a transfer is a sub-transfer and a count is a compound-transfer.

However, 10(4) could be interpreted as suggesting that with "transfer" being a sub-transfer, then if a transfer causes
some candidate to reach a quota, then section 9 is interrupted to do the surplus distribution in 7(4) before 
section 9 is complete. Other jurisdictions typically have explicit language to say the exclusion completes before
a surplus distribution starts. This is weak evidence for "transfer" to mean compound-transfer, and introduces
a new ambiguity.

I interpret 10(2) as applying to a sub-transfer, and 10(4) to require the exclusion under section 9 to complete,
at which point the surplus transfer is done. I do not think that other interpretations are untenable.

The NSWEC takes the opposite interpretation for 10(2), interpreting 10(2) to mean a compound-transfer
(evidence example : City of Albury, count 46). This seems plausible to me. They choose the same interpretation as
me for 10(4).

## What is the analogue of 10(2) for transfers under clause 7?

Clause 7(2) is similar to 10(3) but applies to surplus distributions.

What happens if we consider clause 7 to imply multiple transfers, and one of them causes some candidate
to obtain a quota? Does that candidate become elected at that point, in which case they stop being
a continuing candidate, and future sub-transfers do not consider that candidate to be a continuing candidate?

* If yes, this causes problems, as this changes how many votes are exhausted, which changes the surplus fraction
  computed in 7(4)(a), which possibly needs to be recalculated - which seems strange, as different sub-transfers
  would have different surplus fractions - or not, in which case the number of exhausted votes in the
  compound surplus transfer does not match the number of exhausted votes used in the computation of the surplus
  fraction.
  
* If no, this seems somewhat incompatible with the definition of continuing candidate and elected candidate.

This is a problem with no nice solution other than saying that cause 7 does not have sub-transfers, which seems
incompatible with 9(2)(b)(i) as discussed before.

Saying that a quota is only checked at the end of all sub-transfers under clause 7 seems compatible with
the lack of explicit division of clause 7 into separate transfers.

I consider the least bad solution to this to be to say that reaching a quota is only checked at the
end of all sub-transfers under clause 7. I do not think that other interpretations are untenable or even less reasonable.
In particular, given the similarity in language between 10(2) and 7(2), I do not like this difference
in interpretation.

The NSWEC makes the same interpretation as me for this (evidence example City of Albury, count 47).
This is a more compelling interpretation for them than me given their interpretation of 10(2).

### Multi way ties.

8(4) deals with breaking ties for a transfer of surplus, and 9(5) and 9(6) have very similar language for breaking
ties for exclusions. Both purport to deal with more than 2 candidates, but do not do so unambiguously.

Consider 9(5), "Whenever it becomes necessary to exclude a candidate and two or more candidates have the same number of votes and are lowest on the poll, the one who was lowest on the poll at the last count or transfer at which they had an unequal number of votes is first excluded."

What does "have an unequal count" mean if there are 3 candidates in question? Do all three have to be different? Consider
the case of three candidates tied on 7 votes on count 2. On the prior count, count 1, one candidate still had 7 votes, and the other two had 5 votes each.
Does this count as "having an unequal number of votes"?

* If yes, then we have a problem as 9(6) says "If those candidates have had equal numbers of votes at all preceding counts or transfers, ... " which does not apply, so there is no way of breaking ties.
* If no, then we have a problem as which of the equal two are excluded?

There is an equivalent problem with 8(4).

There are many plausible ways of solving this problem:

* Say that by "unequal" it means all unequal in 9(5), and by "equal" it means at least one pair equal in 9(6). 
* Say that by "unequal" it means has a unique lowest number of votes in 9(5) and by "equal" in 9(6) it means "does not have a unique lowest number of votes" in 9(6)
* Say that if there are a tie for lowest when 9(5) is applied, then 9(5) is applied recursively to just those tied candidates, and 9(6) applies if 9(5) does not result in a unique candidate to exclude. 


The first is arguably the closest to the language, and the least powerful solution, in the sense that it is the least likely to successfully break a tie. It was used in the Federal Senate counting legislation prior to 2 Dec 2021. The last is arguably the least close to the language, and the most powerful solution. It was used in federal senate counting legislation after 2 Dec 2021.

I will choose the first of these as the interpretation (for both 9(5)/(6) and 8(4)), as it seems the closest to the literal language. I don't consider the other choices as untenable.

The NSWEC appears to have chosen the third of these methods (evidence: three way tie resolution at the end of count 29 for City of Campbelltown). This 
is reasonable, and has the advantage of resolving more ties.

#### Does the countback in 8(4) or 9(5) apply to sub-transfers or compound transfers?

There is another ambiguity I did not consider at first. Do clauses 8(4) and 9(5) apply to sub-transfers or compound-transfers?
I interpreted it as sub-transfers (like most other jurisdictions) but the NSWEC seems to have interpreted it as
compound-transfers. Evidence example : City of Albury, count 39,
TIERNAN Jodie was eliminated. At the end of count 38, TIERNAN Jodie was tied on 94 with DOCKSEY Graham.
A subset of votes are shown here:
```text
Count     TIERNAN Jodie   DOCKSEY Graham
37             88             92
...
38.27.23.1     94             93
...
38             94             94
```
The NSWEC decided to exclude TIERNAN Jodie, presumably because of count 37, ignoring the subcounts of 38 such as 38.27.23.1.

The NSWEC's decision seems a plausible interpretation, but has the disadvantage of resolving fewer ties.

### Aggregate value of exhausted votes.

In clause 7, the _surplus fraction_ is defined (7(4)(a)), with denominator equal to 
"the number of votes received by the elected candidate (excluding the aggregate value of any exhausted votes)."
The aggregate value of any exhausted votes is defined in 7(6) as 
`the exhausted votes are to be excluded at the value that the votes were transferred to the candidate.`

This is not at all obvious what it means.

An obvious literal interpretation to a mathematician is that this is the sum, over all exhausted votes, of the transfer
value at which the votes were transferred to the candidate. This is literal, straight forward, unambiguous,
and very tempting. However, this does not take into account the effect of rounding down (by clauses 6(1)(d) or similar), 
and could thus lead to overcounting. 

It could even lead to negative transfer values, if the legislation were interpreted literally. For instance,
suppose the quota was 1000, and candidate A received 800 first preference votes, then later 1 vote with transfer value
0.4 (rounded down to 0 votes by 6(1)(d)), then later 1 vote with transfer value 0.3 (rounded down to 0),
and then later still 300 votes with transfer value 1. This leads to 
a total vote count of 1100, 100 above quota. This makes the numerator of the surplus fraction to be 100. Suppose
all votes other than the one with transfer value 0.3 ended up exhausted at the time that this candidate's surplus
was distributed. That would make the aggregate value of any exhausted votes be 1100.4, and so the denominator
of the surplus fraction would be 1100-1100.4 = -0.4, for a surplus fraction of 100/-0.4 = -250. As -250
is not greater than 1, it would not even be caught by the sanity check in 7(4)(a)! This obviously undesirable
outcome could be circumvented by extending the sanity check and cap to 1 in 7(4)(a) to include the case where the denominator
is less than or equal to zero. The absence of such an explicit sanity check is thus an argument against this
interpretation; of course it may be that the legislators never considered this possibility.

However if we discard the obvious literal straight forward solution, it is not clear what it is replaced by.
Consider the following cases:

* 1 vote got transferred with TV 0.6, and was rounded down to 0. 
  * Should it count as 0.6 (simple interetation)?
  * Or 0 (seems reasonable)?
* 2 votes got transferred with TV 0.6, and was rounded down to 1. One of these two ended up exhausted when 7(6) was applied.
  * Does this vote count as 0.6 (simple interpretation)?
  * Or 0.5 (its share of the actual amount transfered to the candidate)?
  * Or even 0.4 (saying the other vote counted as 0.6, which is plausible, as it will be transferred out with this value)?
  * Or 0 (saying that you round it down to an integer, as in 7(4)(c))?
  * Or 1 (saying that if it was not there, the other vote would have counted as 0)?
  
The first case seems straight forward - make it 0. But the second case is not at all obvious, and I don't like any of the choices. None are indefensible. 

I cannot see any compelling interpretation, and so my best guess as to intention is the simple straight forward
literal interpretation of the aggregate value of any exhausted votes being the sum over all exhausted votes of the
transfer values they were transferred in as (first preferences being considered transfer value 1). 
I choose it as it is the simplest interpretation, and it is also consistent with how the votes for candidates
are treated - it is comparable to considering "exhausted" as a candidate. This brings up the somewhat stretched
question about whether it should also analogously be rounded down to an integer, like candidate votes clause 7(4)(c). 
I choose not to add in this extra step as there is little evidence for it.
Along with this interpretation I add in the discussed sanity check on 7(4)(a) to prevent negative transfer values. 
I am not at all claiming that other interpretations are untenable. There is not nearly enough detail in
`the exhausted votes are to be excluded at the value that the votes were transferred to the candidate.`

This is ambiguous, and could change who gets elected.
I believe the NSWEC uses the same interpretation, although I have not checked quite as carefully as in
other cases.

### When to apply clause 11 Election without reaching quota

Clause 11, "Election without reaching quota", contains a variety of ways of terminating the election. Some apply when
there is one remaining vacancy, some when there are multiple remaining vacancies. All but 11(1) are unnecessary - they
will not change who is elected, but may change the order, and will probably change when the count terminates.

It is not clear when clause 11 is supposed to apply. It seems that cause 11 is invoked whenever a surplus
distribution is done (through 6(2) or 7(5)) or an exclusion is done (through 9(7)). However each of these
explicit evocations has an explicit restriction to there being one remaining vacancy. For instance,
9(7) reads "This clause is subject to clause 11 of this Schedule, and if at any time there is one remaining vacancy which can be filled under that clause, no further exclusion under this clause can be made."

One interpretation of this is to say that this means that clause 11 only thus applies if there is exactly one remaining vacancy.
This seems implausible as that would make 11(3) never apply (so why have it), and 11(1) only apply if there is
1 candidate left, in which case the algorithm would not terminate properly if there were 2 candidates left. So that
is an untenable interpretation.

An alternative interpretation is that it is a clumsy way of saying that 11 applies, and it means "at least one" by "one".
In this case the second half of the clause would be a (somewhat redundant) attempt to make the meaning clearer by
explicitly stating that this clause stops operating when clause 11 applies. This seems to me to be the only tenable
interpretation.

#### Clause 11 and sub-transfers.

However this does not solve all the ambiguity with the timing of clause 11. There is still the question of, if an exclusion or surplus transfer
consists of more than one sub-transfer, can clause 11 apply in the middle of the sub-transfers? That is, if
an exclusion consists of multiple transfers, does clause 11 get checked after a partial transfer?

The federal legislation does apply its closest equivalent to clause 11 in the middle of exclusions, so this is not
an unthinkable interpretation.

Ignoring for the moment what the legislation says, there is good reason to *not* apply clause 11 in the middle of
exclusions. 11(2), 11(3) and 11(4) all refer to counts of continuing candidates, plus undistributed surpluses,
and would need to have an extra term for undistributed excluded votes if applied in the middle of an exclusion.

Clause 9(7) says if cause 11 applies, "no further exclusion under this clause can be made". The use of the word
*exclusion* rather than *transfer* here, coupled with the prior paragraph, means that it is pretty clear that
one cannot apply clause 11 in the middle of an exclusion. One could conceivably interpret this to mean that the
clause 11 check is done after each transfer, but even if the clause 11 is triggered, one still finishes the 
in progress exclusion with no available seats left, but this is perverse, even if occasionally done in other jurisdictions.

For the surplus distribution case it is not nearly as clear. The explicit mention of undistributed surpluses in
11(2), 11(3), and 11(4) makes it plausible, although not compelling, as they would still be mentioned in
either interpretation. The wording of clause 6(2) and 7(5), say that if clause 11 applies, 
"no further transfer under this clause can be made." If it said "no further surplus can be transferred under this clause",
then it would be unambiguous. 

I am going to interpret this as saying that the surplus distribution for that
surplus is finished, but no further surplus can be transferred. The reason is partly for similarity with
the exclusions, and partly as this is arguably the simplest interpretation. It does not make a large difference
anyway; it will not change who gets elected in 11(1),(2) or (3) applies, and 11(4) does not apply
if there are undistributed surpluses. It could change the order of election. Of course all of this
is moot if somehow surplus distributions are all done in one monolithic transfer - see earlier discussion.

The NSWEC seems to use the same interpretation as me on this case. 

### Tracking exhausted votes

Clause 15(1)(f) says that the counter must track `the votes which at some stage become exhausted votes.`

It is not entirely clear what this means. Most jurisdictions, when writing out the distribution of preferences,
keep a tally of where the votes have gone. This includes a column of votes lost to exhaustion (along with a 
similar column of votes lost to rounding). These two, when added to the current number of votes for each candidate, should
sum to the total number of votes, and provides a useful sanity check and explanation for what has happened to
all votes. Presumably this is what is intended by 15(1)f. Note that other jurisdictions' legislation do not require this; 
it is freely done by the electoral commissions (and ConcreteSTV) as it is useful.

When a distribution is done, the number of votes that a candidate gets
is rounded down to an integer. Should this be done also for the exhausted votes column? My assumption is
yes, by analogy to what other electoral commissions generally do.

More complexly, in a surplus transfer, the exhausted votes are "absorbed" into the quota through
subtracting the `aggregate value of any exhausted votes`. However if there are more than a quota 
of such aggregate value, then not all exhausted votes will be so absorbed - only some of them will be.
Presumably the remaining exhausted votes should go into the exhausted votes column, although in the ACT 
which has the possibly closest legislation involving a similar case (made more complex via a last parcel),
such exhausted votes are actually put into the lost due to rounding column. This makes the ACT appear to
lose a large number of votes due to rounding, which possibly explains their recent change to round to
6 decimal digits instead of integers (which of course did not solve the problem). This seems very perverse
and I do not consider it a tenable interpretation.

However, there is still the issue that there are (presumably) multiple rounds in the surplus distribution
for the different transfer values. Some portion of these exhausted votes must be assigned to the
exhausted votes column in each of said rounds. What portions? The most reasonable interpretation
I would say would be to parcel out the exhausted votes in proportion to the proportion of the 
aggregate value of the exhausted votes coming from that particular transfer value. That is, if
the total aggregate value of exhausted votes is AV, the total quota is Q (and AV>Q so there are some
exhausted votes not absorbed by the quota), and the aggregate value of exhausted votes in round i is
AVi, then the number of deemed exhausted votes in round i will be AVi/Av*(AV-Q), rounded down to an integer.

This is a lot of detail to deduce from the phrase `the votes which at some stage become exhausted votes.`
but to some extent it does not matter as the column is only for informational purposes; it does not
change the outcome of the election in any way.

The NSWEC seems to solve this ambiguity by not tracking them. Rather than separating out votes lost
to exhaustion and rounding, they have a *Votes Lost* column, which only gets aggregate values
for compound transfers. This neatly avoids the ambiguities, but does seem to be against the vibe
of the legislation in 15 1(c) and (f). In their defense they do track many of these
things in an even more detailed (although possibly harder to interpret) manner in their
excellent dopfulldetails.xlsx file. 

### Tracking votes for the candidate having a surplus distributed.

Clause 15(1)(a) requires tracking `the number of votes counted for each candidate`. How to do this
is not specified in a surplus distribution. 

Before having a surplus distributed, a candidate has some number of votes T > quota.

After having a surplus distributed, a candidate has a number of votes equal to the quota.

However, if the surplus distribution involves multiple transfer values, there need to be intermediate values.
Ideally this would tick the number of votes down in a way indicative of how many votes were distributed. For exclusions,
this is easy. Each step deals with the votes received on one count, and they are all dealt with, so they are subtracted
at that time. The surplus distribution case is more complex as they are *not* all dealt with at this point - some are 
left with the candidate to get the quota. 

The obvious solution is to interpolate. If the candidate starts off with V votes, then the surplus=V-quota,
and a proportion (V-quota)/V are distributed, so one could just subtract (V-quota)/V * (number of votes received with that
particular transfer value). However, this has two problems:

* It does not take into account the exhausted votes, which absorb some of the quota. If the quota "absorbs" all 
  exhausted votes, then the exhausted votes should not contribute to the reduction at this step. But this can lead
  to the highly non-intuitive situation where the number of votes goes up slightly, as the aggregate value of exhausted votes
  may be larger than the number of votes that went to the candidate with the particular transfer value, due to rounding
  effects. Mathematically this is sensible, but it will be very confusing to people looking at it, which is undesirable.
* It leads to non-integer values for the number of votes for the candidate having the surplus distributed, which are otherwise integers.
  This leads a viewer to be surprised and pay too much attention to a value which is not actually very informative, which is
  unhelpful. As the only reason to track this number thus is to be informative - it doesn't affect who is elected in any way -
  it should be done in a way to maximise information to the user. This can be resolved by the simple expedient of keeping
  track of it as a fraction internally, but just printing the result as a rounded integer.
  
I have adopted the (possibly overly simplistic) method of using the above interpolation, listing the tally for 
the candidate being excluded rounded down to an integer. This
is a reasonably accurate (although ignoring some distortions from exhausted votes), intuitively meaningful method.

Many alternatives are tenable; none of them affect who is elected in any way - this is solely a communication issue.

The NSWEC solves it by not including any figures here. This seems reasonable to me even if they seem to be required
as they are not well defined.

### Typos

9 (2) (a) starts "the total number of ballot-papers of the excluded candidate on which first preferences are recorded and which express a next available preference...".
It is not immediately obvious why one bothers to mention "on which first preferences are recorded". 
A careful examination of context makes it clear that the only sensible thing for this to mean is that this means that the first preference should be for the excluded candidate in question.
This is almost certainly just a typo and will be treated as meaning that.

### Bizarre transfer ordering.

Section 9(2)(b) says that in an exclusion transfers are to be done `in the order of the transfers on which the excluded candidate obtained them`.
This seems pretty unambiguous. However, there is some bizarre ordering done by the NSWEC:

In the City of Albury, Count 42.41.20.5.1 is listed after counts 42.41.20.14.1 and 42.41.20.14.5.1.
This is not the order in which the excluded candidate attained them - count 41.20.5.1, for instance, is listed before 41.20.14.1. 
Similarly count 46.40.30.23.4.1 is listed before 46.40.30.4.1. 
I believe this is because the counts are sorted numerically on the first 3 numbers, but lexicographically on the fourth number.

A very similar thing happens for surplus distributions - compare 47.45.40.4.1 and 47.45.40.37.6.2.1

This is almost certainly a bug in the NSWEC's program. However it is a minor issue, as their interpretations of the 
legislation, taken as a whole, means that the order of sub-transfers never matters for determining who is elected or in what order.

I emulate this bizarre behaviour in the rules `NSWECLocalGov2021`