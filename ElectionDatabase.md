# How to set up an election database

The unit tests (amongst other things) test ConcreteSTV's implementation of various
electoral commissions algorithms against their published distribution of preferences
files. This requires having all these file available. For this, a "database" (flat files downloaded
from the Electoral Commissions' websites). 

For copyright reasons I have not included these files.

If you want to run all the unit tests, set up a directory pair "vote_data/Elections" in the directory you are
running the tests from, or a parent directory thereof.

For each jurisdiction you want tested (currently "Federal" and "ACT"), make a subdirectory therein
with the name of the jurisdiction.

For each year you want tested inside that (currently 2013, 2016 and 2019 for Federal), make a subdirectory
therein with the year.

In that directory, put the files for that jurisdiction/year. The tests should complain if you are missing 
a particular file.
