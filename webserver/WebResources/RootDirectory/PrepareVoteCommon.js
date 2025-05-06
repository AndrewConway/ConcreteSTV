"use strict";

// Contains code used by both PrepareVote.js and WhereDidMyVoteGo.js for entering a vote into a
// ballot paper on the screen.


let metadata = null;
let candidateBoxesZeroBased=null; // array, 1 per candidate.
let partyBoxesZeroBased=null; // array, 1 per candidate.
let hasATLvotes = false;
let hasBTLvotes = false;
let availablePreferenceNumbersATL=[];
let availablePreferenceNumbersBTL=[];
let onVoteChanged = null; // a function that is called when your vote changes with a list of candidate IDs from first preference to last stated preference.

function checkAllNumbersAdjustingSummary() { checkAllNumbers(true); }
function checkAllNumbersNotAdjustingSummary() { checkAllNumbers(false); }
function checkAllNumbers(adjustSummary) {
    /// if we used to have ATL votes, first get rid of any synthetic BTL values created from them.
    if (hasATLvotes) {
        for (let b of candidateBoxesZeroBased) b.value="";
    }
    // first see if entries are ATL or BTL.
    hasATLvotes = false;
    for (let b of partyBoxesZeroBased) if (b.value!=="") hasATLvotes=true;
    for (let b of candidateBoxesZeroBased) b.disabled=hasATLvotes;
    hasBTLvotes=false;
    if (!hasATLvotes) {
        for (let b of candidateBoxesZeroBased) if (b.value!=="") hasBTLvotes=true;
    }
    for (let b of partyBoxesZeroBased) b.disabled=hasBTLvotes;
    // Interpret either the ATL boxes or the BTL boxes, checking for duplicates.
    function interpretBoxes(boxes) {
        let used = [];
        let bad = [];
        let summary = "";
        for (let i=0;i<boxes.length;i++) {
            const v = boxes[i].value;
            if (v!=="") {
                if (used[v]!==undefined) { bad[i]=true; bad[used[v]]=true; }
                else used[v]=i;
            }
            if (i!==0) summary+=",";
            summary+=v;
        }
        const availablePreferenceNumbers = [];
        for (let i=1;i<=boxes.length;i++) { // preference numbers are 1 based.
            if (used[i]===undefined) availablePreferenceNumbers.push(i);
            boxes[i-1].className=bad[i-1]?"bad":"good";
        }
        let candidatesInPreferenceOrder = [];
        for (let i=1;i<=metadata.candidates.length;i++) { // preference numbers are 1 based.
            if ((used[i]!==undefined) && !bad[i]) candidatesInPreferenceOrder.push(used[i]);
            else break;
        }
        return { availablePreferenceNumbers:availablePreferenceNumbers, summary:summary,candidatesInPreferenceOrder:candidatesInPreferenceOrder };
    }
    // now interpret preferences, converting to BTL preferences if needed.
    let proc_atl = interpretBoxes(partyBoxesZeroBased);
    if (hasATLvotes) {
        // convert ATL votes to BTL votes
        for (let b of candidateBoxesZeroBased) b.value="";
        let candidatePreferencesGiven = 0;
        for (const partyIndex of proc_atl.candidatesInPreferenceOrder) {
            for (const candidateIndex of metadata.parties[partyIndex].candidates) {
                candidatePreferencesGiven++;
                candidateBoxesZeroBased[candidateIndex].value=candidatePreferencesGiven;
            }
        }
    }
    let proc_btl = interpretBoxes(candidateBoxesZeroBased);
    availablePreferenceNumbersATL = hasBTLvotes?[]:proc_atl.availablePreferenceNumbers;
    availablePreferenceNumbersBTL = hasATLvotes?[]:proc_btl.availablePreferenceNumbers;
    document.getElementById("likedCandidate").innerText = availablePreferenceNumbersBTL.length===0 ? "None" : availablePreferenceNumbersBTL[0];
    document.getElementById("despisedCandidate").innerText = (availablePreferenceNumbersBTL.length===0) ? "None" : availablePreferenceNumbersBTL[availablePreferenceNumbersBTL.length-1];
    document.getElementById("togoCandidates").innerText = availablePreferenceNumbersBTL.length.toString();
    let lpe = document.getElementById("likedParty");
    if (lpe) lpe.innerText = availablePreferenceNumbersATL.length===0 ? "None" : availablePreferenceNumbersATL[0];
    let dpe = document.getElementById("despisedParty");
    if (dpe) dpe.innerText = (availablePreferenceNumbersATL.length===0) ? "None" : availablePreferenceNumbersATL[availablePreferenceNumbersATL.length-1];
    const tePreferences=document.getElementById("preferences");
    if (adjustSummary) tePreferences.value = proc_btl.summary;
    tePreferences.style.width = (tePreferences.scrollWidth+10) + 'px';
    if (onVoteChanged) {
        onVoteChanged(proc_btl.candidatesInPreferenceOrder);
    }
    const url = document.getElementById("BookmarkableURL");
    if (url) {
        let wanted_url = new URL(window.location.href);
        wanted_url.search = "?"+(new URLSearchParams(hasATLvotes?{atl:proc_atl.summary}:{btl:proc_btl.summary}).toString());
        url.setAttribute("href",wanted_url.href);
    }
}


function setupCandidates(allowPartiesAsWell) {
    function createNumberBoxForCandidate(div,_candidateIndex) { // create the box at the start of a candidate
        let cNumber = add(div,"input");
        cNumber.type="number";
        cNumber.min=1;
        cNumber.max=metadata.candidates.length;
        cNumber.addEventListener("input",checkAllNumbersAdjustingSummary);
        return cNumber;
    }
    function createNumberBoxForParty(div,_partyIndex) { // create the box at the start of a party
        let cNumber = add(div,"input");
        cNumber.type="number";
        cNumber.min=1;
        cNumber.max=metadata.parties.length;
        cNumber.addEventListener("input",checkAllNumbersAdjustingSummary);
        return cNumber;
    }
    function clickOnName(isATL) {
        function clickOnCandidateName(candidateIndex,cNumber,event) { // called when someone clicks on a name
            if (cNumber.value!=="") cNumber.value="";
            else {
                let availablePreferenceNumbers = isATL?availablePreferenceNumbersATL:availablePreferenceNumbersBTL;
                if (availablePreferenceNumbers.length>0) {
                    cNumber.value = availablePreferenceNumbers[event.altKey?availablePreferenceNumbers.length-1:0];
                }
            }
            checkAllNumbersAdjustingSummary();
        }
        return clickOnCandidateName;
    }
    const boxes = drawBallotPaper(true,createNumberBoxForCandidate,clickOnName(false),allowPartiesAsWell?createNumberBoxForParty:null,allowPartiesAsWell?clickOnName(true):null);
    candidateBoxesZeroBased= boxes.candidateBoxes;
    partyBoxesZeroBased= boxes.partyBoxes;
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
    for (let b of partyBoxesZeroBased) b.value="";
    checkAllNumbersNotAdjustingSummary();
}

function loadFromATLPreferenceList(atlPreferences) {
    const entered = atlPreferences.split(",");
    for (let i=0;i<metadata.parties.length;i++) {
        partyBoxesZeroBased[i].value= (i<entered.length)? entered[i].trim() : "";
    }
    for (let b of candidateBoxesZeroBased) b.value="";
    checkAllNumbersAdjustingSummary();
}
