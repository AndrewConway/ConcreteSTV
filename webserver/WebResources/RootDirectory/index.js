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
                add(yearDiv,"span","year").innerText=year.year;
                for (const electorate of year.electorates) {
                    const electorateA = add(yearDiv,"a","electorate");
                    electorateA.innerText=electorate;
                    electorateA.href=source.name+"/"+year.year+"/"+electorate+"/";
                }
            }
            const sourceDiv = add(mainDiv,"dib")
        }
    })
}
