"use strict";

// This file contains a couple of general utilities that Andrew often uses.
// Copyright 2018-2021 Andrew Conway. All rights reserved, but may be distributed under GPL 3.0 or later or other by arrangement.


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
