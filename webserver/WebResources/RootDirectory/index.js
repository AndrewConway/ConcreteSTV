"use strict";


window.onload = function () {
    addHeaderAndFooter();
    getWebJSON("get_all_contests.json",contests=>{
        console.log(contests);
        const mainDiv = document.getElementById("AvailableElections");
        if (contests.Err) add(mainDiv,"h3").innerText="Error : "+contests.Err;
        else for (const source of contests.Ok) {
            console.log(source);
            add(mainDiv,"h4").innerText=source.name;
            const ecDiv = add(mainDiv,"div");
            ecDiv.append("Administered by ");
            const ecA = add(ecDiv,"a");
            ecA.innerText=source.ec_name;
            ecA.href=source.ec_url;
            for (const year of source.years) {
                const yearDiv = add(mainDiv,"div");
                const multiColumn = year.electorates.length>10;
                add(yearDiv,"span","year").innerText=year.year;
                const electoratesHolder = add(yearDiv,"span",multiColumn?"MulticolumnListOfLinks":"");
                for (const electorate of year.electorates) {
                    const electorateA = add(electoratesHolder,"a","electorate");
                    electorateA.innerText=electorate;
                    electorateA.href=source.name+"/"+year.year+"/"+electorate+"/";
                }
            }
            const sourceDiv = add(mainDiv,"dib")
        }
    })
}
