# Ambiguities in the ACT casual vacancy legislation

The ACT counting legislation is generally well written and unambiguous. Indeed it is better in
this regard than most such legislation in Australia IMO. However the ACT casual vacancy legislation
is significantly more ambiguous, although one can reasonably easily make a reasonable choice of interpretations.

My thoughts and implementation decisions are documented below.
The references below are to the Electoral Act 1992, Schedule 4.
Part 4.3 is specific to the casual vacancies, and references to section 11-17 refer to it, earlier
sections to the general legislation in part 4.1 and 4.2.

Remember I am not a lawyer; these are just my rather pedantic thoughts as a reader of English.

## Section 12 Quota ambiguity.

The counting system is basically _Instant Run Off_ (IRV) with weighted votes. 
IRV can be considered a special case of the normal STV legislation just through
setting the quota to be at least half the total number of votes. As long as the
quota is at least half, the winner is the same. So the quota should be largely
irrelevant as long as it is at least half. The only effect will be the point at
which the inevitable winner is called. However, I try to match the legislation
perfectly, and this requires computing the quota.

Section 12 says:
```text
(1) For this part, the quota, in relation to a count, is calculated as follows:
                TVA/2 + 1
(2) In this clause:
TVA means the sum of the total votes allotted to the continuing
candidates at the count, any fraction being disregarded.
```

The ambiguous part here is the parsing of the phrase "any fraction being disregarded".

In the discussion below, assume candidate `i` has `Vi` votes, which will not necessarily be an integer.

* TVA is the rounded down version of the sum over `i` of `Vi`, then the quota is calculated, which may be fractional if TVA is odd.
* TVA is the sum over `i` of the rounded down version of `Vi`, then the quota is calculated, which may be fractional if TVA is odd.
* TVA is the sum over `i` of `Vi`, then the quota is calculated, then the quota is rounded down.
* Multiple round downs may occur.

I will ignore the last as it is the least plausible interpretation of the English, and, besides, 
any combination involving rounding down each `Vi` has the same problem as described in the next paragraph, 
and the remaining combination is equivalent to the third option above.

I reject the second of these also as it is surely not what any sane drafter would have intended as it means the quota could
end up being less than half the continuing votes, and a candidate could get elected when another candidate is preferenced higher
by more than half the voters. 

In the corresponding section 1B of the normal quota, the formula is given in 1B(1), and
then 1B(2) says `However, any fraction is to be disregarded.` This is some vague support for the third, which is otherwise
a somewhat obscure parsing. More strong support for this was the presence of the same language in section 12 in the
2016 version of the legislation, when the vote tallies were integers rather than the 2020 6 decimal places. In this
case the first two interpretations were meaningless (although see discussion of (14) below).

So I interpret section 12 as 
 * TVA is the sum over `i` of `Vi`, then the quota is calculated, then the quota is rounded down to an integer.

Elections ACT in their fact sheet do not address rounding of the transfer value, saying
- "To be
  elected, a candidate must obtain 50% plus 1 (an absolute majority) of the number of votes
  counted to all the contesting candidates remaining in the count (excluding exhausted votes)."


## Section 13 Transfer value ambiguity

This section deals with computing the weight to be assigned to each vote. The general idea is that votes
should not be double counted, so the portion of a vote that went to getting the former MLA election should
be the portion that goes to determining the replacement. 

### Section 13(1) ambiguity

The legislation says:
```text
(1) For this part, the transfer value of ballot papers counted for the former MLA—
    (a) for a ballot paper dealt with at the count at which the former MLA became successful
       —is the value ascertained in accordance with subclause (2) or (3), as the case requires;
    (b) for a ballot paper dealt with at the count under clause 3
       —is 1; and
    (c) for a ballot paper dealt with at any other count
       —is the transfer value of the ballot paper when counted for the purpose of allotting count votes to the former MLA.
```

(1c) is unambiguous. (a) and (b) are ambiguous in the case of first preferences (clause 3). 

What does "become successful" mean? The only reasonable interpretation is the count at which the candidate
became elected (eg by going over quota). An unreasonable alternate interpretation is the count at which the candidate's
surplus is redistributed. This latter interpretation is somewhat supported by the meaning of clause 13(2) or 13(3) as
they deal with removing the component that was redistributed. However, it is both inconsistent with the use
of the word "successful" elsewhere and is not a valid definition 
as many candidates do not have their surplus distributed (possibly the election is over beforehand, possible
they don't have a surplus). So the only reasonable interpretation is the count at which they
become elected. 

However, this means that this count can be the first preference count, in which
case both clause (a) and (b) apply and usually give different answers - (a) gives a fraction, and (b) gives 1.

As in this case all votes have the same transfer value, it doesn't make much difference in who ends up
being the winner. Indeed, if it were not for the existence of rounding in the legislation then it would
be totally irrelevant. Due however to the rounding it is possible, if unlikely, for this difference
to change the outcome of the election. 

On their website Elections ACT have a [fact sheet](
https://www.elections.act.gov.au/__data/assets/pdf_file/0009/831465/CasualVacancies.pdf)
describing the method they use, which uses different ambiguous language :
- "Where the vacating MLA was elected with a quota of votes on first preferences, all the ballot
  papers used in the recount that show further preferences will have the same transfer value."
So this is totally useless in resolving this ambiguity.

I will use the interpretation that case (a) has precedence over case (b) as it comes before it.

### Section 13(4) ambiguity

Section 13(4) defines values used in 13(2) and 13(3).

```text
(4) In subclauses (2) and (3):

NCP means the number of ballot papers counted for the former MLA
at the count at which he or she became successful that did not specify
a next available preference.

TV means the transfer value of a ballot paper when counted at that
count for the purpose of allotting count votes to the former MLA.

Q means the quota for the election at which the former MLA was last
elected.
```

See the above discussion about what "the count at which he or she became successful" means.
Now, consider the phrase "next available preference" which is well defined in most
situations it is used in - in the middle of a count. However, consider the scenario
of a count where candidate A is excluded, and as a result candidates B and C go over quota.
Before the count, candidate A,B,C are continuing. During the count, candidate A is not continuing,
but candidates B and C are. At the end of the count, B and C are elected, and are no longer
continuing. So there are arguably three different time periods in a count with different definitions
of continuing. Which one is intended by 13(4) when talking about candidate B? 
* It is hard to imagine it was intended that candidate A was intended to be continuing.
* It is hard to imagine it was intended for candidate C to be continuing but not B as B and C become
  non-continuing basically simultaneously.
* It is implausible to argue that B is intended to be continuing, given the definition in section (1):
```text
next available preference means the next highest preference recorded
for a continuing candidate on a ballot paper.
```
  If B were still continuing, then all votes would have a next available preference for B, and NCP would always be zero.

So I believe the only plausible interpretation is that A,B and C are all not continuing. This is
reasonably consistent with the definitions of sections 13(2) and 13(3) which are mostly consistent with 
the idea that the transfer value assigned to a vote is the transfer value when counted for the former MLA,
minus the transfer value given to the votes that are further redistributed, which seems the general
approach aimed for by the legislation. However this is not always the case: 
* If two candidates B and C go over quota in count n, and B's surplus is distributed at count n+1, causing candidate D to
  go over quota, and then C's surplus is distributed at count n+2, then candidate D is no longer continuing, and the
  computation of transfer values and continuing papers under 13(4) will be different to the ones that actually 
  "stayed" with C.
* If a candidate is elected under 4(2) (number of positions remaining to be filled equals the number of continuing candidates),
  then that candidate will have no surplus distributed, and so all the votes will remain with them.

The Elections ACT fact sheet says
- "Those ballot papers are allocated a new transfer value
  according to a formula set out in the Electoral Act, which has the effect of giving them a vote
  value equivalent to the amount of votes needed by the vacating MLA at that count to bring his
  or her vote total up to the quota for election."

This is true (unless the candidate is elected with less than a quota under 4(2)), and supports the (pretty unambiguous) intrepretation
that 13(4) applies even for candidates elected under 4(2), where it is relevant if a candidate gets more than a quota.

## Section 14 Recount - first count ambiguities

Section 14 refers to the replacement of section 3 by the new votes. 

### Very weak ambiguity in 14(1) - meaning of "next available preference"

One could make a weak argument that the "next available preference" in sections (13) and (14) should
be the same, and one should start from the point the candidate was excluded. This is counterargued by:
* The definition of "next available preference" (see above)
* For a former MLA elected under 4(2) there would never be any "next available preference"
* The Elections ACT fact sheet gives an example explicitly the other way.

So I strongly interpret this as incorrect, and the "next available preference" starts from
the start again.

### Stronger ambiguity in 14(2) - how to count/round different transfer values

Section 14(2) says:
```text
(2) The count votes for each continuing candidate shall be determined
and allotted to him or her, and each continuing candidate’s total votes
shall be calculated.
```

This does not discuss the case of different transfer values. There are a few plausible
interpretations:
* A similar approach to the one used in section (9), _Votes of excluded candidates_,
  where the different transfer values are listed in different counts, with a round
  down and quota check occurring at each step.
* A similar approach to the one used in section (9), _Votes of excluded candidates_,
  with the exception of the quota check (9)(2)(d).
* All votes are added together in one round, and the rounding down is applied to the
  sum of all count votes rather than once for each transfer value.

All three could result in different candidates being elected, although the latter two
only differ on rounding which is unlikely to be significant. 

The first is quite likely
to give different results to the latter two, and could result in some candidate winning with
far fewer than half the votes, so this seems very unlikely to be the intended interpretation.

The Elections ACT fact sheet does not mention rounding here.

The example in 2021 of the casual vacancy of Alistair Coe supports the second of these
interpretations. I therefore adopt this interpretation.

# Implementation note:

The main effort is extracting the votes. Then counting them, the standard ACT STV algorithm
works fine apart from not recomputing the quota each time. This will not change who wins, it just
means the standard ACT STV algorithm may continue counting slightly beyond the necessary point. 

As this is a minor issue, I have not added complexity by adding new count algorithms that only
apply in this special case. Use the standard ACT STV algorithm, and understand that there is a chance
that there may be some extra irrelevant counts.
