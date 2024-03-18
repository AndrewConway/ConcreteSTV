# Parsing NSW 2021 Local Government data

This data can be found on the [NSW Election Commission](https://www.elections.nsw.gov.au/) website
fairly readily. It is, at the time of writing, at the addresses specified below, but this is likely
to change in the future as it is moved to the [past elections](https://pastvtr.elections.nsw.gov.au/)
results page. 

As always, pay attention to the copyright statements when downloading data! At the time of writing,
the NSW Electroral Commission has a quite permissive license. Kudos to them for this.

Use the `parse_ec_data` program to convert their files to ConcreteSTV's .stv format, with the `NSWLG2021`
data source. Note that in the examples below I am leaving off the path to the command `parse_ec_data`.

E.g. to parse the *City of Albury*'s data, run the command
```bash
parse_ec_data --out Albury.stv NSWLG2021 "City of Albury"
```

This will tell you you need a file `fp-by-grp-and-candidate-by-vote-type.html`. Download it
from the Albury page on the [NSW Election Commission](https://www.elections.nsw.gov.au/) website.
This (and other files that will be subsequently requested) can currently be done with
```bash
wget -O fp-by-grp-and-candidate-by-vote-type.html https://vtr.elections.nsw.gov.au/LG2101/albury/councillor/report/fp-by-grp-and-candidate-by-vote-type
wget -O councillor.html https://vtr.elections.nsw.gov.au/LG2101/albury/councillor
wget -O grp-and-candidates-result.html https://vtr.elections.nsw.gov.au/LG2101/albury/councillor/report/grp-and-candidates-result
wget -O finalpreferencedatafile.zip https://vtr.elections.nsw.gov.au/LG2101/albury/download/finalpreferencedatafile.zip
```

Note that the URLs of these are likely to change in the future, as specified above. Look for them on the
[past elections](https://pastvtr.elections.nsw.gov.au/) website.

This will create the file `Albury.stv` which can be used by concrete_stv to count. 

Note that if you run this
with the `NSWECLocalGov2021` rules (`concrete_stv NSWECLocalGov2021 Albury.stv`) the resulting transcript will
be different from the official detailed transcript. The reason for this is that on count 13, there is a tie
between *PATTINSON Jill* (candidate 27) and *VAN NOORDENNEN Bill* (candidate 43).
This can be seen in the *EC decisions needed* column of the viewer.
This tie is, by legislation, to be determined by lot. ConcreteSTV needs to know what the choice made by 
the NSW Electoral Commission actually was. In the absence of being told, ConcreteSTV chooses to favour
the higher numbered candidate (who is worse of donkey-vote wise). Thus *PATTINSON Jill* is excluded.
In this particular case however, the NSWEC decided (presumably by lot) to exclude *VAN NOORDENNEN Bill*.

You can specify the NSWEC decision in the .stv file by adding a `--tie` flag to `parse_ec_data` with the
affected candidates ordered by increasing luck:
```bash
parse_ec_data --out Albury.stv --tie 43,27 NSWLG2021 "City of Albury"
```


## Problems

You may need `openoffice` installed on your system to read some NSW data. This is used to help decode some of the files in .xlsx format.
