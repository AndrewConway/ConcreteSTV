"use strict";

let metadata = null;
let who_got_votes = null;

function percent(fraction) { return fraction.toLocaleString(undefined,{style: 'percent', minimumFractionDigits: 1, maximumFractionDigits: 1});}

function showTable() {
    const table = document.getElementById("resultTable");
    removeAllChildElements(table);
    const isParties = document.getElementById("isParties").checked;
    let partyStats = isParties?who_got_votes.parties:who_got_votes.candidates;
    const rowNames = isParties?metadata.parties:metadata.candidates;
    if (isParties) rowNames.push({name:"ungrouped"});
    const headingRow1 = add(table,"tr");
    const headingRow2 = add(table,"tr");
    add(headingRow1,"th");
    add(headingRow2,"th");
    function addHead(title) {
        let hs = add(headingRow1,"th")
        hs.setAttribute("colspan","4");
        hs.innerText = title;
        add(headingRow2,"th").innerText="ATL";
        add(headingRow2,"th").innerText="BTL";
        add(headingRow2,"th").innerText="BTL %";
        add(headingRow2,"th").innerText="Total";
    }
    addHead("First Preference");
    addHead("Mentioned at least once");
    let totalATL = 0;
    let totalBTL = 0;
    for (let id=0;id<partyStats.length;id++) {
        const row = add(table,"tr","Striped");
        add(row,"th").innerText=rowNames[id].name;
        function addStats(atl,btl) {
            let total = atl+btl;
            add(row,"td").innerText=atl;
            add(row,"td").innerText=btl;
            add(row,"td").innerText=percent(btl/total);
            add(row,"td","Sum").innerText=total;
        }
        addStats(partyStats[id].first_atl,partyStats[id].first_btl);
        totalATL+=partyStats[id].first_atl;
        totalBTL+=partyStats[id].first_btl;
        addStats(partyStats[id].mention_atl,partyStats[id].mention_btl);
    }
    const lastRow = add(table,"tr","Striped");
    add(lastRow,"th").innerText="Total";
    add(lastRow,"td","Sum").innerText=totalATL;
    add(lastRow,"td","Sum").innerText=totalBTL;
    add(lastRow,"td","Sum").innerText=percent(totalBTL/(totalATL+totalBTL));
    add(lastRow,"td","Sum").innerText=totalATL+totalBTL;

}


window.onload = function () {
    addHeaderAndFooter();
    getMultipleWebJSONResult(["WhoGotVotes.json","metadata.json"],(data,meta)=> {
        who_got_votes=data;
        metadata=meta;
        set_heading_from_metadata(metadata);
        document.getElementById("isParties").addEventListener("input",showTable) ;
        showTable();
    });
}
