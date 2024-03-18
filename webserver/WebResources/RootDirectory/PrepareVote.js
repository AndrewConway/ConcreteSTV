"use strict";

let metadata = null;
let candidateBoxesZeroBased=null; // 1 per candidate.
let availablePreferenceNumbers=[];

function checkAllNumbersAdjustingSummary() { checkAllNumbers(true); }
function checkAllNumbersNotAdjustingSummary() { checkAllNumbers(false); }
function checkAllNumbers(adjustSummary) {
    let used = [];
    let bad = [];
    let summary = "";
    for (let i=0;i<metadata.candidates.length;i++) {
        const v = candidateBoxesZeroBased[i].value;
        if (v!=="") {
            if (used[v]!==undefined) { bad[i]=true; bad[used[v]]=true; }
            else used[v]=i;
        }
        if (i!==0) summary+=",";
        summary+=v;
    }
    availablePreferenceNumbers = [];
    for (let i=1;i<=metadata.candidates.length;i++) { // preference numbers are 1 based.
        if (used[i]===undefined) availablePreferenceNumbers.push(i);
        candidateBoxesZeroBased[i-1].className=bad[i-1]?"bad":"good";
    }
    document.getElementById("likedCandidate").innerText = availablePreferenceNumbers.length===0 ? "None" : availablePreferenceNumbers[0];
    document.getElementById("despisedCandidate").innerText = (availablePreferenceNumbers.length===0) ? "None" : availablePreferenceNumbers[availablePreferenceNumbers.length-1];
    document.getElementById("togoCandidates").innerText = availablePreferenceNumbers.length;
    const tePreferences=document.getElementById("preferences");
    if (adjustSummary) tePreferences.value = summary;
    tePreferences.style.width = (tePreferences.scrollWidth+10) + 'px';
}


function doSearch() {
    const searchString = document.getElementById("preferences").value;
    if(searchString.length===0)  {
        alert("You need to enter your vote in the `Summary' box.");
        return;
    }
    const resultsDiv = document.getElementById("SearchResults")
    const blanksMatchAnything = document.getElementById("blanksMatchAnything").checked;
    function failure(message) {
        resultsDiv.innerHTML="<h5>Error in searching</h5>";
        add(resultsDiv,"p").innerText=message;
    }
    function success(json) {
        if (json.Err) failure(json.Err);
        else {
            const entered = searchString.split(",");
            resultsDiv.innerHTML="<h5>Search for "+searchString+(blanksMatchAnything?" (blanks wild)":"")+"</h5>";
            let best = json.Ok.best;
            let numWildcards = blanksMatchAnything?entered.filter(s=>s==="").length:0;
            if (true) { // show summary
                const summary = add(resultsDiv,"div","resultsSummary");
                add(summary,"em").innerText="Results summary : ";
                let desc = "no votes known. Something is wrong.";
                if (best && best.length>0) {
                    const mismatches1 = metadata.candidates.length-best[0].score;
                    const numMatching1 = best[0].hits.length;
                    const marginToNext = best.length>1?(metadata.candidates.length-best[1].score)-mismatches1:1000;
                    const discdesc = (mismatches1===1)?"one discrepancy":(((mismatches1===0)?"no":mismatches1)+" discrepancies");
                    if (mismatches1>=5) desc="Your vote doesn't seem to be present. Maybe it got declared informal. The closest match had "+mismatches1+" discrepancies.";
                    else if (numMatching1===1) desc = "Assuming the location looks correct, your vote appears to be counted, with a unique best match with "+discdesc+". The next closest match is "+(marginToNext>=10?"at least":"fewer than")+" ten discrepancies away."
                    else if (numMatching1===2) desc = "Assuming the location looks correct, your vote (and that of a friend) appears to be counted, with two best matches with "+discdesc+". The next closest match is "+(marginToNext>=10?"at least":"fewer than")+" ten discrepancies away."
                    else if (mismatches1===0) desc = "Your vote is very popular, you trendsetter. Multiple people voted exactly that way."
                    else desc = "There were multiple votes roughly like yours ("+mismatches1+" discrepancies)."
                }
                add(summary,"span").innerText=desc;
            }
            for (const set of best) {
                let mdiv = add(resultsDiv,"div");
                mdiv.innerHTML="<h6>Matching "+(set.score-numWildcards)+" boxes ("+(metadata.candidates.length-set.score)+" mismatches)</h6>";
                let table = add(mdiv,"table");
                table.innerHTML = "<tr><th>Division</th><th>Polling place</th><th>Ballot</th></td>";
                for (const row of set.hits) {
                    let hrow = add(table,"tr");
                    add(hrow,"td").innerText=row.metadata.Electorate||"";
                    add(hrow,"td").innerText=row.metadata["Collection Point"]||"";
                    let votesd = add(hrow,"td");
                    let row_votes = row.votes.split(",");
                    for (let i=0;i<metadata.candidates.length;i++) {
                        if (i!==0) votesd.append(",");
                        let s = add(votesd,"span");
                        let vote = (i<row_votes.length)?row_votes[i]:"";
                        let desired = (i<entered.length)?entered[i]:"";
                        s.innerText = vote;
                        if (vote !== desired && (desired!=="" || !blanksMatchAnything)) s.className="mismatch";
                    }
                }
                if (set.truncated>0) {
                    add(table,"tr").innerHTML="<td colspan='3' style='text-align: center'>and "+set.truncated+" others.</td>"
                }
            };
            //console.log(json);
        }
    }
    let options = {
        query : searchString,
        blank_matches_anything : blanksMatchAnything
    }
    getWebJSON('find_my_vote',success,failure,JSON.stringify(options),"application/json");
    resultsDiv.innerHTML="<h5><img src='/ajax-loader.gif' alt='Searching'/> Searching...</h5>";
}


function setupCandidates() {
    function createNumberBoxForCandidate(div,candidateIndex) { // create the box at the start of a candidate
        let cNumber = add(div,"input");
        cNumber.type="number";
        cNumber.min=1;
        cNumber.max=metadata.candidates.length;
        cNumber.addEventListener("input",checkAllNumbersAdjustingSummary);
        return cNumber;
    }
    function clickOnName(candidateIndex,cNumber,event) { // called when someone clicks on a name
        if (cNumber.value!=="") cNumber.value="";
        else cNumber.value = availablePreferenceNumbers[event.altKey?availablePreferenceNumbers.length-1:0];
        checkAllNumbersAdjustingSummary();
    }
    candidateBoxesZeroBased=drawBallotPaper(true,createNumberBoxForCandidate,clickOnName)
    document.getElementById("preferences").addEventListener("input",loadFromPreferenceList);
    document.getElementById("preferences").addEventListener("change",checkAllNumbersAdjustingSummary);
    checkAllNumbersAdjustingSummary();
}

/** Called when the preference list is manually edited. Transfers values from it to the ballot */
function loadFromPreferenceList() {
    const tePreferences=document.getElementById("preferences");
    const entered = tePreferences.value.split(",");
    for (let i=0;i<metadata.candidates.length;i++) {
        candidateBoxesZeroBased[i].value= (i<entered.length)? entered[i].trim() : "";
    }
    checkAllNumbersNotAdjustingSummary()
}


window.onload = function () {
    addHeaderAndFooter();
    getWebJSONResult("metadata.json",meta=> {
        metadata=meta;
        set_heading_from_metadata(metadata);
        setupCandidates();
    });
    getWebJSONResult("info.json",info=> {
        if (!info.simple) {
            document.getElementById("SearchDiv").innerText="Election result data is not available yet. Searching is not available yet."
        }
    });
}
