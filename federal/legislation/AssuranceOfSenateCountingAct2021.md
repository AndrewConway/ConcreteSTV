# Changes to Federal Senate vote counting 2021

This is an extract from the [Electoral Legislation Amendment (Assurance of Senate Counting) Act 2021](https://www.legislation.gov.au/Details/C2021A00135)

This is covered by a [Creative Commons Attribution 4.0 International (the CC BY 4.0 licence)](http://creativecommons.org/licenses/by/4.0/)

The full legislation is [included as a pdf](C2021A00135.pdf).
Sourced from the Federal Register of Legislation at 20 May 2022. 
For the latest information on Australian Government law please go to https://www.legislation.gov.au.

This document 
Based on content from the Federal Register of Legislation at 20 May 2022. 
For the latest information on Australian Government law please go to https://www.legislation.gov.au. 

I have changed the formatting and have a subset of the legislation that I consider relevent to ConcreteSTV.
I have inserted headings breaking the changes up by what they affect, and some comments in boxes.

Nothing is intended to in any way imply an endorsement of this project by the Federal Register of Legislation.

This legislation is in response to [our paper](../../reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf)
where we pointed out differences between the legislation and what the AEC actually did; this somewhat 
changes the legislation to match what the AEC did in 2016. Note that sometimes the AEC did something
that seems more sensible than the legislation (e.g. tiebreaking rules, or ignoring the multiple
exclusion legislation), so this is a reasonable response. It introduces a different
counting algorithm (possibly with different results) for manual and computerized counting,
and it treats tie breaking differently for exclusions (countback using any difference),
order of election (countback requiring all different) and tie at end (no countback).

The most important part of this legislation is the audit of the digitization
process, which is not relevant to counting and thus not included here, but which
means there should be actual evidence that the result is correct.
*This is a great thing for Australian democracy*, and I strongly recommend
other jurisdictions adopt similar laws, preferably though making it public.

# Part 2—Counting

Commonwealth Electoral Act 1918

## Changing tie resolution for elections:

### 2 Subsection 273(17)
Omit “shall have a casting vote but shall not otherwise vote at the
election”, substitute “must determine by lot which of those candidates is
to be elected”.

```text
This affects the case where there are 2 continuing candidates and
one vacancy. There is no countback in the case of a tie. This is
unlikely to come up in practice.
```
### 3 Paragraph 273(20)(b)
After “shall determine”, insert “by lot”.
### 4 Subsection 273(22)
After “shall determine”, insert “by lot”.

```text
These two deal with the case where multiple candidates are elected at the
same round with the same number of votes. This is unlikely to come up in
practice. There is a countback, requiring all candidates to have different
tallies, which is different to the tie resolution for exclusions.

Section 20 covers order of election.
Section 22 covers order of surplus distribution.
I am not sure whether these are supposed to be the same if decided by lot.
```

## Changing tie resolution for exclusions.

### 5 Subsection 273(29)
Insert:

_relative order of standing_, at a particular time, of 2 continuing
candidates with the same number of votes in a Senate election for a
State means:
* (a) the relative order of standing of those candidates by reference
to the last count at which they had a different number of
votes, with the candidate with the greater number of votes at
that count having a higher relative order of standing than the
other candidate; or
* (b) if those candidates are in an unbreakable tie at that time—the
relative order of standing of those candidates by reference to
the order of standing determined under subsection (29A) in
relation to the unbreakable tie.

_unbreakable tie_: 2 or more continuing candidates who have the
same number of votes in a Senate election at a particular time are
in an unbreakable tie at that time if:
(a) they had the same number of votes at every count before that
time; or
(b) there was no count before that time.

### 6 After subsection 273(29)
Insert:

(29A) If, at a particular time, 2 or more continuing candidates in a Senate
election for a State are in an unbreakable tie, the Australian
Electoral Officer for the State must determine by lot the order of
standing of those candidates relative to each other at that time.

### 7 Paragraph 273(31)(a)
Omit “paragraph (b)”, substitute “paragraphs (b) and (c)”.

### 8 Paragraph 273(31)(b)
Repeal the paragraph, substitute:
* (b) if 2 continuing candidates have the same number of votes at
  that time—those candidates are to stand in the poll in their
  relative order of standing at that time;
* (c) if 3 or more continuing candidates have the same number of
  votes at that time—those candidates are to stand in the poll in
  the order determined in accordance with subsection (31A).

### 9 After subsection 273(31)
Insert:
* (31A) For the purposes of paragraph (31)(c), if 3 or more continuing
  candidates (the tied candidates) have the same number of votes at
  a particular time, the tied candidates are to stand in the poll in the
  order determined by:
    * (a) identifying each possible combination of 2 tied candidates;
      and
    * (b) for each combination of 2 tied candidates identified under
      paragraph (a), working out the relative order of standing, at
      that time, of those 2 candidates; and
    * (c) ranking all of the tied candidates such that:
        * (i) the tied candidate who has a higher relative order of
          standing, at that time, than each other tied candidate
          stands highest in the poll; and
        * (ii) a tied candidate who has a higher relative order of
          standing, at that time, than another tied candidate stands
          higher in the poll than that other candidate; and
        * (iii) the tied candidate who does not have a higher relative
          order of standing, at that time, than any other tied
          candidate stands lowest in the poll.
```
 This brings the legislation into compliance with the AEC's actions.
 It is a good tie breaking strategy that maximises the ability to resolve ties given the spirit of countbacks.
 
 This seems to apply only to the order for exlusion:
 273 13 (a) the candidate who stands lowest in the poll must be excluded;
```


### 10 Subsection 273A(5)
Omit “subsections 273(8) to (32) (inclusive)”, substitute
“subsections 273(8) to (13AA) and subsections 273(14) to (32)”.
### 11 Subsection 273A(9)
Omit “subsections 273(8) to (30) (inclusive)”, substitute
“subsections 273(8) to (13AA) and subsections 273(14) to (30)”.

```text
This means that if the count is being done electronically (which
it almost certainly is), then don't use the bulk exclusion legislation.

Removing the bulk exclusion legislation is a good idea:
* The AEC did not do it in 2016 and 2019 anyway.
* It is complex, error prone, and buggy.

Leaving it in for manual counting but removing it for electronic counting
seems somewhat undesirable, as it means a different set of elected candidates 
could occur depending on whether it was counted manually or electronically. 
But the odds of this are small and other errors are more likely to affect the result.
```