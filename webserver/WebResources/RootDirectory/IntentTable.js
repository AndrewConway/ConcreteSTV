"use strict";

let metadata = null;
let candidateBoxes = []; // will be one per candidate.

function showTable(data,options) {
    const table = document.getElementById("resultTable");
    removeAllChildElements(table);
    const headingRow = add(table,"tr");
    add(headingRow,"th");
    for (const id of options.who) {
        add(headingRow,"th").innerText=options.who_is_groups?(metadata.parties&&metadata.parties.length>id?metadata.parties[id].name:"ungrouped"):metadata.candidates[id].name;
    }
    add(headingRow,"th").innerText="Exhausted";
    add(headingRow,"th").innerText="Sum";
    for (let id=0;id<data.table.length;id++) {
        const row = add(table,"tr","Striped");
        add(row,"th").innerText=options.first_pref_by_groups?(metadata.parties&&metadata.parties.length>id?metadata.parties[id].name:"ungrouped"):metadata.candidates[id].name;
        for (const entry of data.table[id]) add(row,"td").innerText=entry===0?"":entry;
        add(row,"td","Sum").innerText=sumArray(data.table[id]);
    }
    const sumRow = add(table,"tr","Sum");
    add(sumRow,"th").innerText="Sum";
    for (let col=0;col<1+options.who.length;col++) add(sumRow,"td","Sum").innerText=sumArray(data.table.map(row=>row[col]));
    add(sumRow,"td","Sum").innerText=sumArray(data.table.map(row=>sumArray(row)));

}

function showComputing() {
    document.getElementById("resultTable").innerHTML="<tr><td>"+computingHTML+"</td></tr>";
}

function recomputeTable() {
    let checkedIndices = [];
    candidateBoxes.forEach( (box,index) => { if (box.checked) checkedIndices.push(index); } );
    const options = {
        first_pref_by_groups : document.getElementById("isGroupsRows").checked,
        who_is_groups : document.getElementById("isGroupsColumns").checked,
        use_atl : document.getElementById("useATL").checked,
        use_btl : document.getElementById("useBTL").checked,
        who : checkedIndices,
    };
    showComputing();
    getWebJSONResult(getURL("IntentTable.json",options),data=>showTable(data,options));
}

function redrawMajorPartiesList() {
    candidateBoxes=drawBallotPaper(!document.getElementById("isGroupsColumns").checked,function (div,_index) {
        const input = add(div,"input");
        input.type="checkbox";
        input.addEventListener("input",recomputeTable)
        return input;
    },(_index,box)=> {box.checked=!box.checked;});
}

window.onload = function () {
    addHeaderAndFooter();
    getWebJSONResult("metadata.json",meta=> {
        metadata=meta;
        set_heading_from_metadata(metadata);
        checkGroupsCheckboxForMeaning("isGroupsColumns",metadata,false);
        checkGroupsCheckboxForMeaning("isGroupsRows",metadata,false);
        checkGroupsCheckboxForMeaning("useATL",metadata,false);
        checkGroupsCheckboxForMeaning("useBTL",metadata,true);
        redrawMajorPartiesList();
        document.getElementById("isGroupsColumns").addEventListener("input",function() { redrawMajorPartiesList(); recomputeTable(); }) ;
        document.getElementById("isGroupsRows").addEventListener("input",recomputeTable) ;
        document.getElementById("useATL").addEventListener("input",recomputeTable);
        document.getElementById("useBTL").addEventListener("input",recomputeTable);
        recomputeTable();
    });
}
