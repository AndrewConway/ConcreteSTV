"use strict";

let metadata = null;
let meanVoteData = null;

let showingCandidate = -1;
let candidateBoxes = []; // will be one per candidate.

function setupCandidates() {
    candidateBoxes=drawBallotPaper(true,(div,_index)=>add(div,"span","NumberBox"),setCandidate);
    document.getElementById("showATL").addEventListener("change",function () { setCandidate(showingCandidate); });
}

function setCandidate(candidateIndex) {
    showingCandidate = candidateIndex;
    document.getElementById("showingCandidate").innerText = (showingCandidate===-1)?"Any":metadata.candidates[showingCandidate].name;
    let use2 = document.getElementById("showATL").checked?[meanVoteData.all,meanVoteData.all_by_first_preference]:[meanVoteData.btl,meanVoteData.btl_by_first_preference]
    let use = (showingCandidate===-1)?use2[0]:use2[1][showingCandidate]; // of type MeanPreferenceByCandidate
    for (let i=0;i<candidateBoxes.length;i++) candidateBoxes[i].innerText = use.mean_preference[i].toFixed(1);
    document.getElementById("numATL").innerText = use.num_atl;
    document.getElementById("numBTL").innerText = use.num_btl;
}

function clearCandidate() { if (metadata) setCandidate(-1); }

window.onload = function () {
    addHeaderAndFooter();
    getMultipleWebJSONResult(["MeanPreferences.json","metadata.json"],(mean_prefs,meta) => {
        meanVoteData=mean_prefs;
        metadata=meta;
        set_heading_from_metadata(metadata);
        setupCandidates();
        setCandidate(-1);
    })
}
