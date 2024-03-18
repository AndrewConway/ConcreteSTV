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
        rulesDescription(rulesDiv,info);
    }
    const simpleDiv = add(mainDiv,"div")
    if (info.simple) {
        document.getElementById("ElectionDataAvailableSection").className="";
        if (!info.can_read_raw_markings) {
            document.getElementById("NeedsRawAccess1").className="hidden";
            document.getElementById("NeedsRawAccess2").className="hidden";
        }
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
        add(formal,"td").innerText=info.simple.num_atl;
        add(formal,"td").innerText=info.simple.num_btl;
        const unique = add(statsTable,"tr","Striped");
        add(unique,"th").innerText="Unique";
        add(unique,"td").innerText=info.simple.num_unique_atl+info.simple.num_unique_btl;
        add(unique,"td").innerText=info.simple.num_unique_atl;
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
            add(mainDiv,"div").innerText="* Note that group voting tickets were used, and ATL votes for a party with multiple tickets counts as one unique vote for each ticket type (usually 1, sometimes 2 or 3)."
        }
        if (info.simple.download_locations) {
            for (const location of info.simple.download_locations) {
                const locDiv = add(simpleDiv,"div");
                addMaybeA(locDiv,"Raw files used",location.url);
                locDiv.append(" : "+location.files.join(", "));
            }
        }
    } else {
        document.getElementById("ElectionDataNotAvailableSection").className="";
    }
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
        tieResolutionDescription(metaDiv,metadata,metadata.tie_resolutions);
    }
}


window.onload = function () {
    addHeaderAndFooter();
    getWebJSONResult("info.json",process_good_info);
    getWebJSONResult("metadata.json",process_good_metadata);
}
