"use strict";

function process_good_info(info) {
    const mainDiv = document.getElementById("InfoPlaceholder");
    removeAllChildElements(mainDiv);
    const acknowledgementDiv = add(mainDiv,"div");
    if (info.ec_name&&info.ec_url) {
        acknowledgementDiv.append("Managed by ");
        const a = add(acknowledgementDiv,"a");
        a.href=info.ec_url;
        a.innerText=info.ec_name;
        acknowledgementDiv.append(". ");
    }
    if (info.copyright) {
        if (info.copyright.statement) {
            acknowledgementDiv.append("Derived from data ")
            addMaybeA(acknowledgementDiv,info.copyright.statement,info.copyright.url);
            acknowledgementDiv.append(". ");
        }
        if (info.copyright.license_name) {
            acknowledgementDiv.append("Licensed under ")
            addMaybeA(acknowledgementDiv,info.copyright.license_name,info.copyright.license_url);
            acknowledgementDiv.append(". ");
        }
    }
    if (info.rules) {
        const rulesDiv = add(mainDiv,"div");
        if (info.rules.rules_used) {
            rulesDiv.append("The rules used for this election appear to be ")
            addRules(rulesDiv,info.rules.rules_used);
            rulesDiv.append(". ")
        }
        if (info.rules.rules_recommended) {
            rulesDiv.append("I recommend using ")
            addRules(rulesDiv,info.rules.rules_recommended);
            rulesDiv.append(". ")
        }
        if (info.rules.comment) {
            rulesDiv.append(info.rules.comment);
            rulesDiv.append(" ");
        }
        if (info.rules.reports) {
            for (const report of info.rules.reports) {
                rulesDiv.append("We have written a ")
                addMaybeA(rulesDiv,"report",report);
                rulesDiv.append(" about this election. ");
            }
        }
    }
    const simpleDiv = add(mainDiv,"div")
    if (info.simple) {
        document.getElementById("ElectionDataAvailableSection").className="";
        if (!info.can_read_raw_markings) document.getElementById("NeedsRawAccess").className="hidden";
        add(mainDiv,"h4").innerText="The results are available";
        const statsTable = add(mainDiv,"table","rightalign");
        const headRow = add(add(statsTable,"thead"),"tr");
        add(headRow,"th");
        add(headRow,"th").innerText="Votes";
        add(headRow,"th").innerText="ATL";
        add(headRow,"th").innerText="BTL";
        const formal = add(statsTable,"tr","Striped");
        add(formal,"th").innerText="Formal";
        add(formal,"td").innerText=info.simple.num_formal;
        add(formal,"td").innerText=info.simple.uses_group_voting_tickets?"*":info.simple.num_atl;
        add(formal,"td").innerText=info.simple.num_btl;
        const unique = add(statsTable,"tr","Striped");
        add(unique,"th").innerText="Unique";
        add(unique,"td").innerText=info.simple.num_unique_atl+info.simple.num_unique_btl;
        add(unique,"td").innerText=info.simple.uses_group_voting_tickets?"*":info.simple.num_unique_atl;
        add(unique,"td").innerText=info.simple.num_unique_btl;
        if (info.simple.num_informal) {
            const formal = add(statsTable,"tr","Striped");
            add(formal,"th").innerText="Informal";
            add(formal,"td").innerText=info.simple.num_informal;
            add(formal,"td").innerText="";
            add(formal,"td").innerText="";
        }
        for (const vtype of info.simple.vote_types) {
            const row = add(statsTable,"tr","Striped");
            add(row,"th").innerText=vtype.name;
            add(row,"td").innerText=vtype.num_atl+vtype.num_btl;
            add(row,"td").innerText=vtype.num_atl;
            add(row,"td").innerText=vtype.num_btl;
        }
        if (info.simple.uses_group_voting_tickets) {
            add(mainDiv,"div").innerText="* Note that group voting tickets were used, and ATL ticket votes have been converted into BTL equivalents."
        }
        if (info.simple.download_locations) {
            for (const location of info.simple.download_locations) {
                const locDiv = add(simpleDiv,"div");
                addMaybeA(locDiv,"Raw files used",location.url);
                locDiv.append(" : "+location.files.join(", "));
            }
        }
    } else simpleDiv.append("Election data is not currently available.")
}

function process_good_metadata(metadata) {
    const title = title_from_metadata(metadata);
    document.title=title;
    set_heading_from_metadata(metadata);
    document.getElementById("TitleHeading").innerText=title;
    const metaDiv = document.getElementById("MetadataPlaceholder");
    add(metaDiv,"h4").innerText="The contest";
    add(metaDiv,"div").innerText="There are "+metadata.candidates.length+" candidates"+(metadata.parties?" and "+metadata.parties.length+" groups":"")+".";
    if (metadata.vacancies) add(metaDiv,"div").innerText="There are "+metadata.vacancies+" vacancies to fill."
    if (metadata.enrolment) add(metaDiv,"div").innerText="There are "+metadata.enrolment+" voters enrolled."
    if (metadata.tie_resolutions) {
        add(metaDiv,"h4").innerText="Tie resolutions by lot";
        for (const tie of metadata.tie_resolutions) {
            if (Array.isArray(tie)) add(metaDiv,"div").innerText=tie.map(n=>metadata.candidates[n].name).join(" was favoured over ");
            else if (tie.favoured) {
                add(metaDiv,"div").innerText=tie.favoured.map(n=>metadata.candidates[n].name).join(" and ")+(tie.favoured.length>1?" were ":" was ")+"favoured over "+tie.disfavoured.map(n=>metadata.candidates[n].name).join(" and ")+(tie.came_up_in?" around count "+tie.came_up_in:"")+".";
            }
        }
    }
}


window.onload = function () {
    addHeaderAndFooter();
    getWebJSONResult("info.json",process_good_info);
    getWebJSONResult("metadata.json",process_good_metadata);
}