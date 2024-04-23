"use strict";

// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

let document_to_show = null;

function zero_is_blank(n) { return (n===0 || n===null ||n===undefined)?"":n.toString(); }
function delta(newV,oldV) {
    if (!newV) newV=0;
    if (!oldV) oldV=0;
    const diff = newV-oldV;
    if (diff===0) return "";
    let res = diff.toString();
    if (res.length>Math.max(newV.toString().length,oldV.toString().length)+2) {
        // probably a rounding issue. Round to 6 decimal places which is the longest currently supported.
        res=diff.toFixed(6);
        if (res.includes(".")) { // delete trailing zeros, in case res is an integer.
            while (res.endsWith("0")) res=res.substring(0,res.length-1);
            if (res.endsWith(".")) res=res.substring(0,res.length-1);
        }
    }
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

function effect_to_string(metadata,list) {
    return list.map(c=>metadata.candidates[c].name).join(" & ");
}

function SayLoading() {
    const render_div = document.getElementById("RenderThingToView");
    removeAllChildElements(render_div);
    render_div.innerText="Reading file. Please wait."
}
function Render() {
    // general purpose clearing up.
    const render_div = document.getElementById("RenderThingToView");
    removeAllChildElements(render_div);
    const heading = document.getElementById("MainHeading");
    heading.innerHTML="";
    if (!document_to_show) return;
    const metadata = document_to_show.metadata || document_to_show.original.metadata;
    if (metadata&&metadata.name) { // compute the name
        const name = metadata.name;
        const heading_text = name.year.toString()+" "+name.name+" : "+name.electorate+(name.modifications?(" "+name.modifications.join(" ; ")):"");
        heading.innerText=heading_text;
    }
    if (metadata&&metadata.name&&metadata.name.comment) {
        add(render_div,"div","comment").innerText=metadata.name.comment;
    }
    // work out what to show.
    document.getElementById("TranscriptOnly").className="hidden";
    document.getElementById("ChangeOnly").className="hidden";
    document.getElementById("NothingChosen").className="hidden";
    if (document_to_show.transcript) {
        document.getElementById("TranscriptOnly").className="";
        const has_set_aside = document_to_show.transcript.counts.some(a=>a.set_aside_for_quota);
        document.getElementById("ShowSetAside").className=has_set_aside?"":"hidden";
        document.getElementById("ShowSetAsideLabel").className=has_set_aside?"":"hidden";
        RenderTranscript(document_to_show,render_div);
    } else if (document_to_show.original && document_to_show.changes) {
        document.getElementById("ChangeOnly").className="";
        RenderChanges(document_to_show,render_div);
    } else {
        document.getElementById("NothingChosen").className="";
    }
}

/** Display a list of changes */
function RenderChanges(vote_changes_document,render_div) {
    const vote_data = vote_changes_document.original;
    const metadata = vote_data.metadata;
    const changes_div = add(render_div,"div");
    const changes_table = add(changes_div,"table","changes");
    const table_head = add(add(changes_table,"thead"),"tr");
    const table_body = add(changes_table,"tbody");
    add(table_head,"th").innerText="Ballots";
    add(table_head,"th").innerText="Effect";
    add(table_head,"th").innerText="How";
    add(table_head,"th").innerText="Details";
    const num_atl=vote_data.atl.length;
    // vote_changes_document.changes.sort((a,b)=>a.ballots.n-b.ballots.n);
    for (const change of vote_changes_document.changes) {
        const tr = add(table_body,"tr");
        add(tr,"td").innerText=change.ballots.n;
        const effect_td = add(tr,"td");
        add(effect_td,"div").innerText="+ "+effect_to_string(metadata,change.outcome.list1only);
        add(effect_td,"div").innerText="- "+effect_to_string(metadata,change.outcome.list2only);
        let howbox = add(tr,"td");
        let how_is_empty = true;
        function how(text,description) {
            if (how_is_empty) how_is_empty=false;
            else howbox.appendChild(document.createTextNode(", "));
            let entry = add(howbox,"abbr");
            entry.innerText=text;
            entry.title=description;
        }
        if (change.requires.changed_first_preference) how("First Preferences","At least one first preference vote was changed or removed");
        if (change.requires.changed_atl) how("ATL","At least one above the line ballot was affected");
        if (change.requires.added_ballots) how("Added","At least one ballot was added");
        if (change.requires.removed_ballots) how("Removed","At least one ballot was removed");
        if (change.requires.changed_ballots) how("Changed","At least one ballot was changed");
        if (change.requires.affected_verifiable_ballots) how("Affected Verifiable","At least one ballot that is in principle considered verifiable was affected");
        if (change.requires.directly_benefited_new_winner) how("Direct to beneficiary","At least one modification directly went to a candidate (or party for ATL) who ended up elected as a result");
        if (change.requires.directly_hurt_new_loser) how("Direct from victim","At least one modification directly came from a candidate (or party for ATL) who ended up not elected as a result");
        let details = add(tr,"td");
        for (const sc of change.ballots.changes) {
            let text = sc.n+" ("+sc.tally+" votes)";
            if (sc.from) {
                text+=" TV "+sc.from.tv;
                text+=" "+metadata.candidates[sc.from.candidate].name;
            }
            if ((typeof sc.candidate_to)==="number") {
                text+=" → "+metadata.candidates[sc.candidate_to].name;
            } else {
                text+=" removed"
            }
            let detailsdiv = add(details,"div","hoverable");
            detailsdiv.innerText=text;
            if (sc.from) {
                const tooltip = add(detailsdiv,"div","tooltip");
                for (const b of sc.from.ballots) {
                    let vote = "";
                    let votetype = undefined;
                    if (b.from<num_atl) {
                        vote="ATL "+vote_data.atl[b.from].parties.join(",");
                        votetype = vote_data.atl_types && vote_data.atl_types.find(t=>t.first_index_inclusive<=b.from && b.from<t.last_index_exclusive);
                    } else {
                        vote="BTL "+vote_data.btl[b.from-num_atl].candidates.join(",");
                        votetype = vote_data.btl_types && vote_data.btl_types.find(t=>t.first_index_inclusive<=b.from-num_atl && b.from-num_atl<t.last_index_exclusive);
                    }
                    if (votetype) vote=votetype.vote_type+" "+vote;
                    add(tooltip,"div").innerText=b.n+"× "+vote;
                }
            }
        }
    }
}
/** Display a distribution of preferences transcript */
function RenderTranscript(full_transcript,render_div) {
    let heading_orientation = document.getElementById("heading-orientation").value;
    let show_papers = document.getElementById("ShowPapers").checked;
    let show_set_aside = document.getElementById("ShowSetAside").checked;
    const metadata = full_transcript.metadata;
    const transcript = full_transcript.transcript;
    const rounding_ever_used = transcript.counts.some(c=>c.status.papers.rounding || c.status.tallies.rounding);
    const exhausted_ever_used = transcript.counts.some(c=>c.status.papers.exhausted || c.status.tallies.exhausted);
    if (transcript.quota) {
        const above_table = add(render_div,"div","quota");
        above_table.innerText="Quota : "+transcript.quota.quota+" Votes with first preference : "+transcript.quota.papers+" Vacancies : "+transcript.quota.vacancies;
    }
    const table = add(render_div,"table");
    const elected_list = add(render_div,"div","WinningCandidatesList");
    add(elected_list,"h4").innerText="Winning Candidates";
    const FirstCandidate = new Set(); // First candidate in a party
    const LastCandidate = new Set(); // Last candidate in a party
    let people_before_parties = heading_orientation==="slanted";
    let people_row = people_before_parties?add(table,"tr"):null;

    if (metadata.parties && metadata.parties.length>0) {
        let party_row = add(table,"tr");
        add(party_row,"td");
        let num_candidates_done = 0;
        for (const party of metadata.parties) {
            // assume party.candidates is a contiguous sequence, after any previous ones.
            if (party.candidates[0]>num_candidates_done) { // there were some candidates not part of a party in between
                add(party_row,"td","PartyName").colSpan=(party.candidates[0]-num_candidates_done)*(1+(show_papers?1:0)+(show_set_aside?1:0));
            }
            let td = add(party_row,"td","PartyName");
            td.innerText=party.abbreviation||party.name;
            td.colSpan=party.candidates.length*(1+(show_papers?1:0)+(show_set_aside?1:0));
            num_candidates_done=party.candidates[party.candidates.length-1]+1;
            FirstCandidate.add(party.candidates[0]);
            LastCandidate.add(party.candidates[party.candidates.length-1]);
        }
        add(party_row,"td");
    }
    if (!people_row) people_row = add(table,"tr");
    const count_label = add(people_row,"th");
    count_label.innerText="Count";
    const candidate_class_name = heading_orientation==="horiz"?"CandidateNameHoriz":heading_orientation==="vert"?"CandidateNameVert":"CandidateNameSlanted";
    function name_td(extra_class) {
        const td = add(people_row,"td",candidate_class_name+(extra_class?extra_class:""));
        return { td:td, text: add(add(td,"div"),"div")};
    }
    for (let i=0;i<metadata.candidates.length;i++) {
        const candidate = metadata.candidates[i];
        let td = name_td((FirstCandidate.has(i)?" FirstCandidate":"")+(LastCandidate.has(i)?" LastCandidate":""));
        td.text.innerText=candidate.name;
        if (show_papers||show_set_aside) td.td.colSpan=1+(show_papers?1:0)+(show_set_aside?1:0);
    }
    if (exhausted_ever_used) {
        const exhausted_name_td = name_td();
        exhausted_name_td.text.innerText="Exhausted";
        if (show_papers||show_set_aside) exhausted_name_td.td.colSpan=1+(show_papers?1:0)+(show_set_aside?1:0);
    }
    if (rounding_ever_used) {
        name_td().text.innerText="Rounding";
    }
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
        let count_desc = count.count_name || count_number.toString();
        fullSpanTD("count_no").innerText=count_desc;
        count_name_by_id.push(count_desc);
        for (const e of count.elected) {
            elected.add(e.who);
            add(elected_list,"div","WinningCandidate").innerText=""+(elected.size)+". "+metadata.candidates[e.who].name;
        }
        for (const nc of count.not_continuing) not_continuing.add(nc);
        for (let i=0;i<metadata.candidates.length;i++) {
            const status = elected.has(i)?"Elected":not_continuing.has(i)?"Eliminated":"Continuing"
            const delta_td = deltarow?add(deltarow,"td",status):null;
            if (delta_td) delta_td.innerText=delta(count.status.tallies.candidate[i],last_count.status.tallies.candidate[i]);
            const td = add(row,"td",status);
            if (show_papers) {
                if (deltarow) {
                    add(deltarow,"td",status+" BallotPapers").innerText=delta(count.status.papers.candidate[i],last_count.status.papers.candidate[i]);
                }
                const papers = add(row,"td",status+" BallotPapers");
                papers.innerText=zero_is_blank(count.status.papers.candidate[i]);
            }
            if (show_set_aside) {
                if (deltarow) {
                    add(deltarow,"td",status+" SetAside").innerText=count.set_aside_for_quota?zero_is_blank(count.set_aside_for_quota.candidate[i]):"";
                }
                const papers = add(row,"td",status);
            }
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
        if (exhausted_ever_used) {
            if (deltarow) add(deltarow,"td","Continuing").innerText=delta(count.status.tallies.exhausted,last_count.status.tallies.exhausted);
            add(row,"td","Continuing").innerText=zero_is_blank(count.status.tallies.exhausted);
            if (show_papers) {
                if (deltarow) add(deltarow,"td","Continuing BallotPapers").innerText=delta(count.status.papers.exhausted,last_count.status.papers.exhausted);
                add(row,"td","Continuing BallotPapers").innerText=zero_is_blank(count.status.papers.exhausted);
            }
            if (show_set_aside) {
                if (deltarow) add(deltarow,"td","Continuing SetAside").innerText=count.set_aside_for_quota?zero_is_blank(count.set_aside_for_quota.exhausted):"";
                add(row,"td","Continuing");
            }
        }
        if (rounding_ever_used) {
            if (deltarow) add(deltarow,"td","Continuing").innerText=delta(count.status.tallies.rounding,last_count.status.tallies.rounding);
            add(row,"td","Continuing").innerText=zero_is_blank(count.status.tallies.rounding);
        }
        const tv_td = fullSpanTD("TransferValue");
        tv_td.innerText=format_transfer_value(count.created_transfer_value&&count.created_transfer_value.transfer_value || count.portion.transfer_value,count.created_transfer_value?null:format_from(count.portion.when_tv_created));
        if (count.created_transfer_value) {
            let title = "Surplus : "+count.created_transfer_value.surplus+" Ballots considered : "+count.created_transfer_value.ballots_considered+" continuing : "+count.created_transfer_value.continuing_ballots;
            if (count.created_transfer_value.original_transfer_value) title+=" original transfer value : "+count.created_transfer_value.original_transfer_value;
            if (count.created_transfer_value.multiplied_transfer_value) title+=" common multiple "+count.created_transfer_value.multiplied_transfer_value;
            if (count.created_transfer_value.excluded_exhausted_tally) title+=" exhausted tally "+count.created_transfer_value.excluded_exhausted_tally;
            tv_td.title=title;
        }
        fullSpanTD("CountAction").innerText=count.reason==="FirstPreferenceCount"?"First Preference Count":count.reason.hasOwnProperty("ExcessDistribution")?"Surplus distribution for "+cname(count.reason.ExcessDistribution):"Exclusion of "+count.reason.Elimination.map(cname).join(" & "); // TODO prettify
        function candidate_index_array_to_string(candidate_list) {
            return candidate_list.map(candidate=>metadata.candidates[candidate].name+" ("+candidate+")").join(",");
        }
        function text_description_of_decision(a) {
            if (a.affected) return candidate_index_array_to_string(a.affected); // deprecated old style, left for compatibility with old transcripts.
            else {
                return a.increasing_favour.map(candidate_index_array_to_string).join(" < ");
            }
        }
        fullSpanTD("ECDecisions").innerText=count.decisions.map(text_description_of_decision).join(" and ");
        fullSpanTD("FromCount").innerText=count.portion.papers_came_from_counts.map(format_from).join(", ");
        count_number+=1;
        last_count=count;
    }
    // fix up height of diagonal columns.
    if (heading_orientation==="slanted") {
        let max_height = 0;
        for (const e of document.querySelectorAll("td.CandidateNameSlanted > div > div")) {
            max_height=Math.max(max_height,e.getBoundingClientRect().height);
        }
        people_row.style.height=""+Math.ceil(max_height)+"px";
    }
}

/** Received a list of possible URLs to choose from for data.
 *  The list should be in the format { title : "New window title", choices : [ "name1.vchange", "name2.vchange" ]}.
 *  Consider "ls -Q -m" to produce this file.
 */
function GotURLList(baseURL,list) {
    if (list.title) document.title = list.title;
    const box = document.getElementById("ChooseBox");
    box.className = "";
    for (const choice of list.choices) {
        const option = add(box,"option");
        option.value = choice;
        option.innerText = choice.replace(/.vchange$/," vote changes");
    }
    document.getElementById("NothingChosen").innerText="Please choose the document you wish to display"
    function changeFunction() {
        let url = new URL(box.value,baseURL);
        function failure(message) { document.getElementById("NothingChosen").innerText="Could not load "+url.toString()+" because "+message; }
        function success(data) { document_to_show=data; Render(); }
        getWebJSON(url.toString(),success,failure);
    };
    box.onchange = changeFunction;
    changeFunction();
}
function ChooseTranscript() {
    const files = document.getElementById("ChooseTranscript").files;
    if (files.length>0) {
        SayLoading();
        files[0].text().then(text=>{document_to_show=JSON.parse(text); Render(); });
    }
}

function MainViewerOnLoadFunction() {
    const url = new URL(document.location.href);
    const provided_list_url = url.searchParams.get("list");
    if (provided_list_url) { // fetch a list of options from the list
        const resolved_provided_list_url = new URL(provided_list_url,url);
        function failure(message) { document.getElementById("NothingChosen").innerText="Could not load "+resolved_provided_list_url.toString()+" because "+message; }
        getWebJSON(resolved_provided_list_url.toString(),list => GotURLList(resolved_provided_list_url,list),failure);
        document.getElementById("ChooseTranscript").className="hidden";
    } else { // access local files
        document.getElementById("ChooseTranscript").onchange = ChooseTranscript;
    }
    document.getElementById("ShowPapers").onchange = Render;
    document.getElementById("ShowSetAside").onchange = Render;
    document.getElementById("heading-orientation").onchange = Render;
    // function got_std(data) { document_to_show=data; Render(); }
    // getWebJSON("../transcript.json",data=>{full_transcript=data; Render();},null);
}

function triggerDownload (imgURI,filenamebase,extension) {
    const evt = new MouseEvent('click', {
        view: window,
        bubbles: false,
        cancelable: true
    });
    const a = document.createElement('a');
    a.setAttribute('download',filenamebase+(extension||".svg"));
    a.setAttribute('href', imgURI);
    a.setAttribute('target', '_blank');
    a.dispatchEvent(evt);
}
function saveHTML(name,title) {
    const desired_section = document.getElementById(name);
    const desired_section_text = desired_section.outerHTML;
    let internal_css = "";
    for (const css of document.getElementsByTagName("link")) {
        if (css.sheet && css.sheet.cssRules) {
            for (const rule of css.sheet.cssRules) {
                internal_css+=rule.cssText+"\n";
            }
        }
    }
    const full_string = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"/><title>"+title+"</title><style>"+internal_css+"</style></head><body>"+desired_section_text+"</body></html>";
    const DOM_URL = window.URL || window.webkitURL || window;
    const blob = new Blob([full_string], {type: 'text/html;charset=utf-8'});
    const url = DOM_URL.createObjectURL(blob);
    triggerDownload(url,title,".html");
}
