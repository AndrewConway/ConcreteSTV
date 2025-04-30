"use strict";

const use_exact_numbers = false;
const digits_for_transfer_values = 3;

let currently_showing_transcript = null;
// Display where the vote went in the div with id "resultsDiv".
// Called when the preference list changes. Argument is a list of candidate IDs, ranked by preference.
function showWhereVoteWent(preferenceList) {
    let div = document.getElementById("resultsDiv");
    removeAllChildElements(div);
    add(div,"h2").innerText="Where did my vote listing consecutive preferences for "+preferenceList.length+" candidates go?";
    const table = add(div,"table");
    const head = add(add(table,"thead"),"tr");
    add(head,"th").innerText="Count";
    add(head,"th").innerText="Reason";
    add(head,"th").innerText="Candidate vote sitting at";
    add(head,"th").innerText="Value";
    const body = add(table,"tbody");
    let my_preference_upto = 0;
    let transfer_value = 1.0;
    let transfer_value_text = "1";
    let transfer_value_ratio_text = "1";
    const full_svg_width = 400;
    const normal_svg_height = 30;
    function make_simple_transfer_value_picture(where,isExhausted,loses) {
        const wins = loses===false;
        const svg=addSVG(where,"svg");
        svg.setAttribute("width",full_svg_width*2);
        svg.setAttribute("height",normal_svg_height);
        const rect = addSVG(svg,"rect",wins?"transfer_value_used":loses?"transfer_value_continuing_loses":isExhausted?"transfer_value_continuing_exhausted":"transfer_value_continuing");
        rect.setAttribute("x",full_svg_width-transfer_value*full_svg_width)
        rect.setAttribute("y",0);
        rect.setAttribute("width",transfer_value*full_svg_width);
        rect.setAttribute("height", normal_svg_height);
        const text = addSVG(svg,"text","transfer_value_continuing");
        text.textContent = transfer_value_text+(wins?" elected":loses?" not elected":isExhausted?" exhausted":"");
        text.setAttribute("x",full_svg_width-transfer_value*full_svg_width+2);
        text.setAttribute("y",normal_svg_height/2);
    }
    const not_continuing = new Set(metadata.excluded);
    const not_stuck_on = new Set(metadata.excluded); // should be same as not_continuing, except for elected candidates pending surplus distribution when multiple candidates are elected at the same time.
    for (let count_no=0;count_no<currently_showing_transcript.counts.length;count_no++) {
        const count = currently_showing_transcript.counts[count_no];
        if (count.not_continuing) for (const nc of count.not_continuing) not_continuing.add(nc);
        if (count.reason.Elimination) for (const nc of count.reason.Elimination) not_stuck_on.add(nc);
        if (count.reason.ExcessDistribution!==undefined) not_stuck_on.add(count.reason.ExcessDistribution);
        let changed = count_no===0;
        let svg_drawer = make_simple_transfer_value_picture;
        let description = null;
        if (my_preference_upto<preferenceList.length && count.reason.ExcessDistribution===preferenceList[my_preference_upto]) {
            const ctv = count.created_transfer_value;
            const old_transfer_value = transfer_value;
            transfer_value=ctv.transfer_value;
            transfer_value_text = transfer_value.toString();
            transfer_value_ratio_text = transfer_value_text;
            if (transfer_value_text.includes("/")) {
                let split = transfer_value_text.split("/")
                if (split.length===2) {
                    transfer_value = parseInt(split[0])/parseInt(split[1]);
                    transfer_value_text=(use_exact_numbers?transfer_value_text+" ≈ ":"")+transfer_value.toPrecision(digits_for_transfer_values);
                }
            }
            const transfer_value_used = old_transfer_value-transfer_value;
            function make_surplus_transfer_value_picture(where,isExhausted) {
                const svg=addSVG(where,"svg");
                svg.setAttribute("width",full_svg_width*2);
                svg.setAttribute("height",normal_svg_height*2);
                const x_start_continuing = full_svg_width*(1.0-transfer_value);
                const x_finished_last = x_start_continuing-transfer_value_used*full_svg_width;
                const x_start_used = Math.min(x_start_continuing,x_finished_last); // transfer_value_used may be negative
                const rect_used = addSVG(svg,"rect",transfer_value_used<0?"transfer_value_used_negative":"transfer_value_used");
                rect_used.setAttribute("x",x_start_used)
                rect_used.setAttribute("y",0);
                rect_used.setAttribute("width",Math.abs(x_start_continuing-x_finished_last));
                rect_used.setAttribute("height", normal_svg_height);
                const rect_continuing = addSVG(svg,"rect",isExhausted?"transfer_value_continuing_exhausted":"transfer_value_continuing");
                rect_continuing.setAttribute("x",x_start_continuing)
                rect_continuing.setAttribute("y",normal_svg_height);
                rect_continuing.setAttribute("width",transfer_value*full_svg_width);
                rect_continuing.setAttribute("height", normal_svg_height);
                const text_used = addSVG(svg,"text","transfer_value_used");
                text_used.textContent = "Used "+transfer_value_used.toPrecision(digits_for_transfer_values)+" to elect "+metadata.candidates[count.reason.ExcessDistribution].name;
                text_used.setAttribute("x",2+x_start_used);
                text_used.setAttribute("y",normal_svg_height/2);
                const text_continuing = addSVG(svg,"text","transfer_value_continuing");
                text_continuing.textContent = "Surplus "+transfer_value_text+(isExhausted?" exhausted":"");
                text_continuing.setAttribute("x",2+x_start_continuing);
                text_continuing.setAttribute("y",normal_svg_height*3/2);
            }
            svg_drawer=make_surplus_transfer_value_picture;
            description = metadata.candidates[preferenceList[my_preference_upto]].name+" was elected with "+ctv.votes+" votes giving a surplus of "+ctv.surplus+" producing a transfer value for the surplus votes of "+ctv.surplus+"/"+ctv.votes+". Ballot papers in their pile will be transferred to the next candidate on their preference list who is not already excluded or elected, but only counting as a partial vote of value specified by the transfer value.";
        }
        if (my_preference_upto<preferenceList.length && not_stuck_on.has(preferenceList[my_preference_upto])) {
            while (my_preference_upto<preferenceList.length && not_continuing.has(preferenceList[my_preference_upto])) {
                my_preference_upto++;
                changed=true;
            }
        }
        if (changed) {
            let count_name = count.count_name || (""+(count_no+1));
            let count_reason = count.reason==="FirstPreferenceCount"?"First preference count":count.reason.Elimination?"Exclusion of "+(count.reason.Elimination.map(n=>metadata.candidates[n].name).join(", ")):"Surplus distribution for elected candidate "+metadata.candidates[count.reason.ExcessDistribution].name;
            if (!description) {
                if (count.reason==="FirstPreferenceCount") {
                    description="A pile is created for each candidate containing the ballot papers of those who gave that candidate their first preference.";
                } else if (count.reason.Elimination) {
                    description="When candidates are excluded, the ballots in their pile get transferred to the next candidate on your preference list who is not already excluded or elected.";
                }
            }
            const tr = add(body,"tr");
            add(tr,"td","left").innerText=count_name;
            const reason_td = add(tr,"td","left");
            reason_td.innerText=count_reason;
            reason_td.title=description;
            const candidate = add(tr,"td","left");
            if (my_preference_upto===preferenceList.length) candidate.innerText="Exhausted";
            else {
                add(candidate,"span","number_in_ballot_box").innerText=""+(my_preference_upto+1);
                add(candidate,"span").innerText=" "+metadata.candidates[preferenceList[my_preference_upto]].name;
            }
            let tvd = add(tr,"td","left");
            //tvd.innerText=transfer_value_text;
            svg_drawer(tvd,my_preference_upto===preferenceList.length);
            if (count_no===currently_showing_transcript.counts.length-1 && count.reason.Elimination && my_preference_upto!==preferenceList.length) {
                if (currently_showing_transcript.rules==="AEC2019") {
                    console.log(count.portion);
                    let qualification = add(tvd,"div");
                    qualification.append("(actually vote not transferred, possibly due to a ");
                    const link = add(qualification,"a");
                    link.href="https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf";
                    link.innerText="counting bug";
                    link.title="See section 4.2 of said report. This bug cannot change who is elected, only the order of election."
                    qualification.append(")");
                } else if (currently_showing_transcript.rules==="AEC2013" || currently_showing_transcript.rules==="AEC2016" && count.portion && count.portion.transfer_value) {
                    let used_tv = count.portion.transfer_value;
                    let used_tv_number = 1;
                    if (used_tv.includes("/")) {
                        let split = used_tv.split("/")
                        if (split.length===2) {
                            used_tv_number = parseInt(split[0])/parseInt(split[1]);
                        }
                    }
                    if (used_tv_number>transfer_value) {
                        add(tvd,"div").innerText="(actually vote not transferred due to termination of the count under subsection 18 of the legislation)";
                    }
                }
            }
        }
        if (my_preference_upto===preferenceList.length) break;
    }
    if (my_preference_upto!==preferenceList.length) {
        const tr = add(body,"tr");
        const desc = add(tr,"td","left");
        let was_elected = currently_showing_transcript.elected.includes(preferenceList[my_preference_upto]);
        desc.setAttribute("colspan",3);
        desc.innerText="At the end of the counting "+metadata.candidates[preferenceList[my_preference_upto]].name+(was_elected?" was elected.":" was not elected.");
        make_simple_transfer_value_picture(add(tr,"td","left"),false,!was_elected);
    }
}



window.onload = function () {
    addHeaderAndFooter();
    getWebJSONResult("metadata.json",meta=> {
        metadata=meta;
        set_heading_from_metadata(metadata);
        getWebJSONResult("info.json",info=> {
            if (info.simple && info.rules && (info.rules.rules_used || info.rules.rules_recommended)) {
                if (is_where_did_my_vote_go_supported(info)) {
                    setupCandidates();
                    let search = window.location.search;
                    if (search) {
                        let params = new URLSearchParams(search);
                        let btl = params.get("btl");
                        if (btl) {
                            document.getElementById("preferences").value=btl;
                            loadFromPreferenceList();
                        }
                    }
                    let options = {
                        rules:info.rules.rules_used || info.rules.rules_recommended,
                        candidates_to_be_elected: metadata.vacancies,
                    };
                    getWebJSON("recount",recount_result=>{
                        if (recount_result.Ok) {
                            currently_showing_transcript=recount_result.Ok.transcript;
                            onVoteChanged = showWhereVoteWent;
                            checkAllNumbersNotAdjustingSummary();
                        } else standardFailureFunction(recount_result.Err);
                    },standardFailureFunction,JSON.stringify(options),"application/json");
                } else {
                    document.getElementById("resultsDiv").innerText="Sorry, this page does not support the rules used in this election."
                }
            } else {
                document.getElementById("resultsDiv").innerText="Election result data is not available yet. Where your vote went is not available yet."
            }
        });
    });

}
