"use strict";

let full_transcript = null;

function zero_is_blank(n) { return (n===0 || n===null ||n===undefined)?"":n.toString(); }
function delta(newV,oldV) {
    if (!newV) newV=0;
    if (!oldV) oldV=0;
    const diff = newV-oldV;
    if (diff===0) return "";
    let res = diff.toString();
    if (diff>0) res="+"+res;
    return res;
}
function format_transfer_value(tv,from) {
    if (!tv) return "";
    let res = tv.toString();
    if (res.includes("/")) {
        let split = res.split("/")
        if (split.length===2) {
            const ratio = parseInt(split[0])/parseInt(split[1]);
            res+=" ≈"+ratio.toPrecision(6);
        }
    }
    if (from) { res+=" created "+from;}
    return res;
}
function Render() {
    const render_div = document.getElementById("render");
    removeAllChildElements(render_div);
    const heading = document.getElementById("MainHeading");
    heading.innerHTML="";
    if (!full_transcript) return;
    const metadata = full_transcript.metadata;
    if (metadata&&metadata.name) { // compute the name
        const name = metadata.name;
        const heading_text = name.year.toString()+" "+name.name+" : "+name.electorate+(name.modifications?(" "+name.modifications.join(" ; ")):"");
        heading.innerText=heading_text;
    }
    const transcript = full_transcript.transcript;
    const above_table = add(render_div,"div");
    above_table.innerText="Quota : "+transcript.quota.quota+" Formal Votes : "+transcript.quota.papers+" Vacancies : "+transcript.quota.vacancies;
    const table = add(render_div,"table");
    let party_row = add(table,"tr");
    let people_row = add(table,"tr");
    add(party_row,"td");
    add(people_row,"th").innerText="Count";
    const FirstCandidate = new Set();
    const LastCandidate = new Set();
    for (const party of metadata.parties) {
        let td = add(party_row,"td","PartyName");
        td.innerText=party.abbreviation||party.name;
        td.colSpan=party.candidates.length;
        FirstCandidate.add(party.candidates[0]);
        LastCandidate.add(party.candidates[party.candidates.length-1]);
    }
    add(party_row,"td");
    for (let i=0;i<metadata.candidates.length;i++) {
        const candidate = metadata.candidates[i];
        let td = add(people_row,"td","CandidateName"+(FirstCandidate.has(i)?" FirstCandidate":"")+(LastCandidate.has(i)?" LastCandidate":""));
        td.innerText=candidate.name;
    }
    add(people_row,"th").innerText="Exhausted";
    add(people_row,"th").innerText="Rounding";
    add(people_row,"th").innerText="Transfer Value";
    add(people_row,"th").innerText="Count action";
    add(people_row,"th").innerText="EC decisions needed";
    add(people_row,"th").innerText="From Count";
    let count_number = 1;
    let elected = new Set();
    let not_continuing = new Set();
    let count_name_by_id = [];
    function format_from(from_id) { return (typeof from_id === "number")?count_name_by_id[from_id]:null; }
    function cname(id) { return metadata.candidates[id].name; }
    let last_count = null;
    for (const count of transcript.counts) {
        const deltarow = last_count?add(table,"tr","delta"):null;
        const row = add(table,"tr",count.reason_completed?"MajorEndCount":"MinorEndCount");
        function fullSpanTD(classname) {
            if (deltarow) { const res = add(deltarow,"td",classname); res.rowSpan=2; return res; }
            else { return add(row,"td",classname); }
        }
        let count_desc = count_number.toString();
        fullSpanTD("count_no").innerText=count_desc;
        count_name_by_id.push(count_desc);
        for (const e of count.elected) elected.add(e.who);
        for (const nc of count.not_continuing) not_continuing.add(nc);
        for (let i=0;i<metadata.candidates.length;i++) {
            const status = elected.has(i)?"Elected":not_continuing.has(i)?"Eliminated":"Continuing"
            const delta_td = deltarow?add(deltarow,"td",status):null;
            if (delta_td) delta_td.innerText=delta(count.status.tallies.candidate[i],last_count.status.tallies.candidate[i]);
            const td = add(row,"td",status);
            let tally = count.status.tallies.candidate[i];
            let text = zero_is_blank(tally);
            td.innerText=text;
            if (elected.has(i)) { // see if elected this round
                for (const e of count.elected) if (e.who===i) {
                    let happy = addStart(td,"span");
                    happy.innerText="👑";
                    happy.title = e.why; // TODO make prettier.
                }
            } else { // see if eliminated this round
                if (count.not_continuing.includes(i)) {
                    addStart(delta_td||td,"span").innerText="👎";
                }
            }
        }
        if (deltarow) add(deltarow,"td").innerText=delta(count.status.tallies.exhausted,last_count.status.tallies.exhausted);
        add(row,"td").innerText=zero_is_blank(count.status.tallies.exhausted);
        if (deltarow) add(deltarow,"td").innerText=delta(count.status.tallies.rounding,last_count.status.tallies.rounding);
        add(row,"td").innerText=zero_is_blank(count.status.tallies.rounding);
        const tv_td = fullSpanTD();
        tv_td.innerText=format_transfer_value(count.created_transfer_value&&count.created_transfer_value.transfer_value || count.portion.transfer_value,format_from(count.portion.when_tv_created));
        if (count.created_transfer_value) {
            let title = "Surplus : "+count.created_transfer_value.surplus+" Ballots considered : "+count.created_transfer_value.ballots_considered+" continuing : "+count.created_transfer_value.continuing_ballots;
            if (count.created_transfer_value.original_transfer_value) title+=" original transfer value : "+count.created_transfer_value.original_transfer_value;
            tv_td.title=title;
        }
        fullSpanTD().innerText=count.reason==="FirstPreferenceCount"?"First Preference Count":count.reason.hasOwnProperty("ExcessDistribution")?"Excess distribution for "+cname(count.reason.ExcessDistribution):"Elimination of "+count.reason.Elimination.map(cname).join(" & "); // TODO prettify
        fullSpanTD().innerText=count.decisions.map(a=>a.affected.map(candidate=>metadata.candidates[candidate].name+" ("+candidate+")").join(",")).join(" and ");
        fullSpanTD().innerText=count.portion.papers_came_from_counts.map(format_from).join(", ");
        count_number+=1;
        last_count=count;
    }
}

function ChooseTranscript() {
    const files = document.getElementById("ChooseTranscript").files;
    if (files.length>0) files[0].text().then(text=>{full_transcript=JSON.parse(text); Render(); });
}

window.onload = function () {
    document.getElementById("ChooseTranscript").onchange = ChooseTranscript;
    document.getElementById("ShowPapers").onchange = Render;
    function got_std(data) { full_transcript=data; Render(); }
    getWebJSON("../transcript.json",data=>{full_transcript=data; Render();},null);
}