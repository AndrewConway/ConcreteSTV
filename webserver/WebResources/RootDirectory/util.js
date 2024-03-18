"use strict";

// This file contains a couple of general utilities that Andrew often uses.
// Copyright 2018-2022 Andrew Conway. All rights reserved, but may be distributed under GPL 3.0 or later or other by arrangement.


/** Add a new node of type addWhat to DOM element addTo, returning the new element. If the third argument is present, the new object is assigned that class. */
function add(addTo,addWhat,className) {
    let res = document.createElement(addWhat);
    addTo.appendChild(res);
    if (className) res.setAttribute("class",className);
    return res;
}

/** Add a new node of type addWhat to DOM element addTo, returning the new element. If the third argument is present, the new object is assigned that class. */
function addStart(addTo,addWhat,className) {
    let res = document.createElement(addWhat);
    addTo.prepend(res);
    if (className) res.setAttribute("class",className);
    return res;
}


const svgNS = "http://www.w3.org/2000/svg";


/** Add a new node of type addWhat to DOM element addTo, returning the new element. If the third argument is present, the new object is assigned that class. */
function addSVG(addTo,addWhat,className) {
    let res = document.createElementNS(svgNS,addWhat);
    addTo.appendChild(res);
    if (className) res.setAttribute("class",className);
    return res;
}

/** like addSVG, except at start. */
function prependSVG(addTo,addWhat,className) {
    let res = document.createElementNS(svgNS,addWhat);
    addTo.prepend(res);
    if (className) res.setAttribute("class",className);
    return res;
}


function removeAllChildElements(box) {
    while (box.firstChild) {
        box.removeChild(box.firstChild);
    }
}

/** Call a web service to get some JSON.
 * @param url{string} : The url to call
 * @param success{function} : A callback on success, taking the parsed json as argument
 * @param failure{function} : A callback on error, taking the error message as argument. Optional.
 * @param message{string?} : The message to send, in the case of POST. null/non-existant for GET.
 * @param contentType{string?} : Optional content type to send. Typically "multipart/form-data" or "application/x-www-form-urlencoded". Only meaningful for POST.
 */

function getWebJSON(url,success,failure,message,contentType) {
    const xhr = new XMLHttpRequest();
    xhr.open(message?"POST":"GET",url, true);
    xhr.responseType = 'json';
    if (contentType) xhr.setRequestHeader("Content-Type", contentType);
    xhr.onreadystatechange = function() { // Call a function when the state changes.
        if (this.readyState === XMLHttpRequest.DONE) {
            if (this.status === 200) success(xhr.response);
            else { if (failure) failure(xhr.statusText) }
        }
    }
    if (failure) xhr.onerror = function() { failure(xhr.statusText); }
    if (message) xhr.send(message); else xhr.send();
}

/** Make a GET url from a base (e.g. "foo") and some query data (e.g. {bar:42}). The examples would return "foo?bar=42". */
function getURL(urlBase,queryData) {
    let res = urlBase;
    let sep = "?"
    for (const key in queryData) if (queryData.hasOwnProperty(key)) {
        res+=sep+encodeURIComponent(key)+"="+encodeURIComponent(queryData[key]);
        sep="&";
    }
    return res;
}

/** Retrieve /header.html and /footer.html and insert at the top and bottom of the body section respectively. */
function addHeaderAndFooter() {
    function getFragment(url,success) {
        const xhr = new XMLHttpRequest();
        xhr.open("GET",url, true);
        xhr.responseType = 'document';  // just get as string.
        xhr.onreadystatechange = function() { // Call a function when the state changes.
            if (this.readyState === XMLHttpRequest.DONE) {
                if (this.status === 200) success(xhr.responseXML.body.children);
                else { console.log(xhr.statusText); }
            }
        }
        xhr.onerror = function() { console.log(xhr.statusText); }
        xhr.send();
    }
    getFragment("/header.html",(fragment)=>{ document.body.prepend(...fragment); })
    getFragment("/footer.html",(fragment)=>{ document.body.append(...fragment); })
}


/// add some text, possibly with a href around it.
function addMaybeA(div,text,href) {
    if (href) {
        const a = add(div,"a");
        a.innerText=text;
        a.href=href;
    } else div.append(text);
}

/// Print a message to a div with id "ErrorMessages", creating it if not present
function standardFailureFunction(message) {
    let errorDiv = document.getElementById("ErrorMessages");
    if (!errorDiv) {
        errorDiv=document.createElement("div");
        errorDiv.id="ErrorMessages";
        document.body.prepend(errorDiv)
    }
    add(errorDiv,"h1").innerText="Error";
    add(errorDiv,"div").innerText=message;
}

/// Like getWebJSON, but the returned JSON is a Rust Result. Convert an Err result to a failure, and extract the Ok field for a good result.
/// Use standardFailureFunction if failure not given.
function getWebJSONResult(url,success,failure) {
    if (!failure) failure=standardFailureFunction;
    function real_success(result) {
        if (result.Err) failure(result.Err);
        else if (result.Ok) success(result.Ok);
        else failure("Received uninterpretable data.");
    }
    getWebJSON(url,real_success,failure);
}

/// Like getWebJSONResult, but takes an array of urls.
/// Calls success with one arg per url of the results when all have returned.
function getMultipleWebJSONResult(urls,success,failure) {
    let togo = urls.length;
    let res = [];
    for (let i=0;i<urls.length;i++) {
        res.push(null);
        function partialSuccess(data) {
            res[i]=data;
            togo--;
            if (togo==0) success(...res);
        }
        getWebJSONResult(urls[i],partialSuccess,failure);
    }
}


function getMax(a){
    return Math.max(...a.map(e => Array.isArray(e) ? getMax(e) : e));
}
function getMin(a){
    return Math.min(...a.map(e => Array.isArray(e) ? getMin(e) : e));
}
