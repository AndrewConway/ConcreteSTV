"use strict";

let metadata = null;
let meanVoteData = null;

let showingCandidate = -1;
let candidateBoxes = []; // will be one per candidate.

function setupCandidates() {
    let groupBoxes = []; // map from candidate group index to div
    const paperDiv = document.getElementById("paperDiv");
    removeAllChildElements(paperDiv); // get rid of loading message
    if (metadata.parties) for (const group of metadata.parties) {
        const groupDiv = add(paperDiv,"div","group");
        groupBoxes.push(groupDiv);
        add(groupDiv,"h4").innerText=group.column_id;
        add(groupDiv,"h5").innerText=group.name;
    }
    let ungrouped_box = null;
    if (metadata.candidates.some(c=> !c.hasOwnProperty("party"))) { // there exist ungrouped candidates
        ungrouped_box = add(paperDiv,"div","group");
        add(ungrouped_box,"h4").innerText="Ungrouped";
    }
    for (const candidate of metadata.candidates) {
        const candidateIndex = candidateBoxes.length;
        const cDiv=add(candidate.hasOwnProperty("party")?groupBoxes[candidate.party]:ungrouped_box,"div","CandidateAndNumber");
        candidateBoxes.push(add(cDiv,"span","NumberBox"));
        const cName=add(cDiv,"span");
        cName.innerText = candidate.name;
        cName.addEventListener("click",function () { setCandidate(candidateIndex); });
    }
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
