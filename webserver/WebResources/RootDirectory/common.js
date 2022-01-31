"use strict";

// Utilities used by many pages in ConcreteSTV server, but not useful outside it.
// Copyright 2022 Andrew Conway. All rights reserved, but may be distributed under AGPL 3.0 or later or other by arrangement.


/// Get the title of the election from metadata
function title_from_metadata(metadata) { return metadata.name.electorate+" "+metadata.name.year+" "+metadata.name.name; }

/// set the element "TitleHeading" with appropriate title from the metadata.
function set_heading_from_metadata(metadata) {
    document.getElementById("TitleHeading").innerText=title_from_metadata(metadata);
}

/// Draw a copy of the ballot paper into the div#paperDiv, removing anything that was there.
/// showCandidates : if true, list candidates. Functions below refer to candidates. Otherwise functions below refer to parties
///
/// createX : Create the X before the candidate name. Function taking two args. First is the div that the X should be created in; the second is the index associated with this element. Return the thing.
/// clickOnName: Function called when the name after the X is clicked on. First arg is the index, secondly is the X returned by createX, third is the event of the action
/// returns a list of all the "X"s added.
function drawBallotPaper(showCandidates,createX,clickOnName) {
    let groupBoxes = []; // map from candidate group index to div
    const paperDiv = document.getElementById("paperDiv");
    removeAllChildElements(paperDiv); // get rid of loading message
    let allXs = [];
    function centralPurpose(parent_div,name) {
        const index = allXs.length;
        const cDiv=add(parent_div,"div","CandidateAndNumber");
        const x = createX(cDiv,index);
        allXs.push(x);
        const cName=add(cDiv,"span");
        cName.innerText = name;
        if (clickOnName) cName.addEventListener("click",function (event) { clickOnName(index,x,event); });
    }
    if (metadata.parties) for (const group of metadata.parties) {
        const groupDiv = add(paperDiv,"div","group");
        groupBoxes.push(groupDiv);
        add(groupDiv,"h4").innerText=group.column_id;
        if (showCandidates) {
            add(groupDiv,"h5").innerText=group.name;
        } else {
            centralPurpose(groupDiv,group.name);
        }
    }
    let ungrouped_box = null;
    if (metadata.candidates.some(c=> !c.hasOwnProperty("party"))) { // there exist ungrouped candidates
        ungrouped_box = add(paperDiv,"div","group");
        add(ungrouped_box,"h4").innerText="Ungrouped";
        if (!showCandidates) centralPurpose(ungrouped_box,"");
    }
    if (showCandidates) for (const candidate of metadata.candidates) {
        const candidateIndex = candidateBoxes.length;
        centralPurpose(candidate.hasOwnProperty("party")?groupBoxes[candidate.party]:ungrouped_box,candidate.name);
    }
    return allXs;
}

const computingHTML = '<img src="/ajax-loader.gif"/> Computing...';

function sumArray(a) { return a.reduce((a,b)=>a+b); }