"use strict";

function drawTable(data) {
    const body = document.getElementById("resultsBody");
    removeAllChildElements(body);
    for (let i=0;i<data.ok_up_to.length;i++) {
        const tr = add(body,"tr","Striped");
        add(tr,"th").innerText=i;
        add(tr,"td").innerText=data.ok_up_to[i];
        if (i===0) {
            add(tr,"td");
            add(tr,"td");
            add(tr,"td");
        } else {
            add(tr,"td").innerText=data.repeated[i-1];
            add(tr,"td").innerText=data.repeated_papers[i-1];
            add(tr,"td").innerText=data.missing[i-1];
        }
    }
    // console.log(data);
}

window.onload = function () {
    addHeaderAndFooter();
    getMultipleWebJSONResult(["RepeatedNumbers.json","metadata.json"],(data,metadata) => {
        set_heading_from_metadata(metadata);
        drawTable(data);
    });
}
