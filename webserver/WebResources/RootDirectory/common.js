"use strict";

// Utilities used by many pages in ConcreteSTV server, but not useful outside it.
// Copyright 2022 Andrew Conway. All rights reserved, but may be distributed under AGPL 3.0 or later or other by arrangement.


/// Get the title of the election from metadata
function title_from_metadata(metadata) { return metadata.name.electorate+" "+metadata.name.year+" "+metadata.name.name; }

/// set the element "TitleHeading" with appropriate title from the metadata.
function set_heading_from_metadata(metadata) {
    document.getElementById("TitleHeading").innerText=title_from_metadata(metadata);
}
