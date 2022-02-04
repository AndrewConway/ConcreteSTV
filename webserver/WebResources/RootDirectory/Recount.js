"use strict";

let candidateBoxes=null;
let voteDataAvailable=false;
let metadata = null;
let rules_list = null;

let currently_wanted_options = null;
let currently_showing_transcript = null;

let ec_tie_resolutions_being_edited = [];

function doRecount() {
    const options = getOptions();
    if (options!==null && options!==currently_wanted_options) {
        currently_wanted_options=options;
        currently_showing_transcript = null;
        let div = document.getElementById("RecountOptionsDescriptions");
        removeAllChildElements(div);
        let desc_div = add(div,"div");
        desc_div.append("Recount using ");
        addRules(desc_div,options.rules);
        desc_div.append(" to elect "+options.candidates_to_be_elected+" candidates.")
        for (const candidate in options.ineligible_candidates) {
            desc_div.append(" "+metadata.candidates[candidate].name+" is deemed ineligible.")
        }
        if (options.tie_resolutions && options.tie_resolutions.length>0) {
            tieResolutionDescription(desc_div,metadata,options.tie_resolutions);
        }
        const results = document.getElementById("RenderThingToView");
        results.innerHTML=computingHTML;
        const transcriptControls = document.getElementById("TranscriptViewControls");
        transcriptControls.className = "hidden";
        getWebJSON("recount",recount_result=>{
            if (options===currently_wanted_options) {
                if (recount_result.Ok) {
                    currently_showing_transcript=recount_result.Ok;
                    removeAllChildElements(results);
                    transcriptControls.className = "";
                    RenderTranscript(currently_showing_transcript,results);
                } else standardFailureFunction(recount_result.Err);
            }
        },standardFailureFunction,JSON.stringify(options),"application/json");
    }
}

function getOptions() {
    if (!voteDataAvailable) return null;
    let ineligible_candidates = [];
    for (let i=0;i<candidateBoxes.length;i++) if (candidateBoxes[i].checked) ineligible_candidates.push(i);
    let selected_rule_index = document.getElementById("RulesChoice").selectedIndex;
    let rules = selected_rule_index===-1 ? null: rules_list&&rules_list[selected_rule_index].name;
    if (rules===null) return null;
    return {
        excluded:ineligible_candidates,
        rules:rules,
        candidates_to_be_elected: +document.getElementById("NumVacancies").value,
        tie_resolutions:ec_tie_resolutions_being_edited||[], // TODO have a way to choose this.
    }
}


function checkCouldRecompute() {
    const options = getOptions();
    const button = document.getElementById("RecountButton");
    button.disabled = options===null;
}


function changeRules() {
    let selected = document.getElementById("RulesChoice").selectedIndex;
    let description = selected===-1 ? "": rules_list&&rules_list[selected].description;
    document.getElementById("RulesDescription").innerText=description;
    checkCouldRecompute();
}

function process_good_info(info) {
    removeAllChildElements(document.getElementById("appropriateRules"));
    if (info.rules) {
        rulesDescription(document.getElementById("appropriateRules"),info);
    }
    if (info.simple) {
        voteDataAvailable=true;
        checkCouldRecompute();
    } else document.getElementById("whyNotCount").innerText="Vote data not available"
    getWebJSON("/rules.json",rules => {
        rules_list=rules;
        let select = document.getElementById("RulesChoice");
        let index = 0;
        let recommended_index = -1;
        let actual_index = -1;
        for (const r of rules) {
            add(select,"option").append(r.name);
            if (info.rules.rules_recommended===r.name) recommended_index=index;
            if (info.rules.rules_used===r.name) actual_index=index;
            index+=1;
        }
        if (recommended_index!==-1) select.selectedIndex = recommended_index;
        else if (actual_index!==-1) select.selectedIndex = actual_index;
        select.addEventListener("input",changeRules)
        changeRules();
    },standardFailureFunction);
}

function redraw_tie_resolutions() {
    let div = document.getElementById("TieResolutionsDisplay");
    removeAllChildElements(div);
    let count = 0;
    for (const tie of ec_tie_resolutions_being_edited) {
        const index = count;
        count++;
        const line = add(div,"div");
        const removeButton = add(line,"button");
        removeButton.innerText="❌";
        removeButton.onclick=()=>{ ec_tie_resolutions_being_edited.splice(index,1); redraw_tie_resolutions(); }
        line.append(" "+descriptionOfSingleTie(metadata,tie));
    }
}

let numFavourChoices = 1;
function addFavourChoice() {
    if (!metadata) return;
    numFavourChoices++;
    const where = add(document.getElementById("extraFavourChoices"),"span");
    where.id = "FavouredCandidateSpan"+numFavourChoices;
    where.append(" and ");
    let select = add(where,"select");
    select.id="FavouredCandidate"+numFavourChoices;
    for (const candidate of metadata.candidates) {
        add(select,"option").append(candidate.name);
    }
    select.addEventListener("input",checkFavouredVisibility);
    document.getElementById("RemoveFavourChoiceButton").className="";
}

function removeFavourChoice() {
    if (numFavourChoices===1) return;
    document.getElementById("FavouredCandidateSpan"+numFavourChoices).remove();
    numFavourChoices--;
    if (numFavourChoices===1) document.getElementById("RemoveFavourChoiceButton").className="hidden";
}

function getNewTieBeingCurrentlyEdited() {
    let favoured = [];
    for (let i=1;i<=numFavourChoices;i++) {
        const candidate = document.getElementById("FavouredCandidate"+i).selectedIndex;
        if (candidate!== -1) favoured.push(candidate);
    }
    if (favoured.length===0) return null;
    const candidateDisfavoured = document.getElementById("DisFavouredCandidate").selectedIndex;
    if (candidateDisfavoured=== -1) return null;
    return {favoured:favoured, disfavoured: [candidateDisfavoured]};
}

function addFavour() {
    const tie = getNewTieBeingCurrentlyEdited();
    if (tie!==null) {
        ec_tie_resolutions_being_edited.push(tie);
        redraw_tie_resolutions();
    }
}

function checkFavouredVisibility() {
    document.getElementById("AddFavour").disabled = getNewTieBeingCurrentlyEdited()===null;
}

function process_good_metadata(_metadata) {
    metadata=_metadata;
    set_heading_from_metadata(metadata);
    candidateBoxes=drawBallotPaper(true,function (div,_index) {
        const input = add(div,"input");
        input.type="checkbox";
        input.addEventListener("input",checkCouldRecompute)
        return input;
    },(_index,box)=> {box.checked=!box.checked;});
    if (metadata.excluded) for (const candidate of metadata.excluded) candidateBoxes[candidate].checked=true;
    document.getElementById("NumVacancies").value=metadata.vacancies;
    ec_tie_resolutions_being_edited=metadata.tie_resolutions || [];
    redraw_tie_resolutions();
    const candidateFavoured = document.getElementById("FavouredCandidate1");
    const candidateDisfavoured = document.getElementById("DisFavouredCandidate");
    for (const candidate of metadata.candidates) {
        add(candidateFavoured,"option").append(candidate.name);
        add(candidateDisfavoured,"option").append(candidate.name);
    }
    candidateFavoured.addEventListener("input",checkFavouredVisibility);
    candidateDisfavoured.addEventListener("input",checkFavouredVisibility);
    getWebJSONResult("info.json",process_good_info);
}


window.onload = function () {
    addHeaderAndFooter();
    getWebJSONResult("metadata.json",process_good_metadata);
}
