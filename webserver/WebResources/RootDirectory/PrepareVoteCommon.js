"use strict";

// Contains code used by both PrepareVote.js and WhereDidMyVoteGo.js for entering a vote into a
// ballot paper on the screen.


let metadata = null;
let candidateBoxesZeroBased=null; // 1 per candidate.
let availablePreferenceNumbers=[];
let onVoteChanged = null; // a function that is called when your vote changes with a list of candidate IDs from first preference to last stated preference.

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
    document.getElementById("togoCandidates").innerText = availablePreferenceNumbers.length.toString();
    const tePreferences=document.getElementById("preferences");
    if (adjustSummary) tePreferences.value = summary;
    tePreferences.style.width = (tePreferences.scrollWidth+10) + 'px';
    if (onVoteChanged) {
        let candidatesInPreferenceOrder = [];
        for (let i=1;i<=metadata.candidates.length;i++) { // preference numbers are 1 based.
            if ((used[i]!==undefined) && !bad[i]) candidatesInPreferenceOrder.push(used[i]);
            else break;
        }
        onVoteChanged(candidatesInPreferenceOrder);
    }
    const url = document.getElementById("BookmarkableURL");
    if (url) {
        let wanted_url = new URL(window.location.href);
        wanted_url.search = "?"+(new URLSearchParams({btl:summary}).toString());
        url.setAttribute("href",wanted_url.href);
    }
}


function setupCandidates() {
    function createNumberBoxForCandidate(div,_candidateIndex) { // create the box at the start of a candidate
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
    candidateBoxesZeroBased=drawBallotPaper(true,createNumberBoxForCandidate,clickOnName).candidateBoxes;
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
