// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use statistics::dendrogram::Dendrogram;

#[test]
fn test_dendrograms_using_matrix_in_wikipedia() {
    // Wikipedia has articles on dendrogram clustering with examples for a specific matrix.
    let distance_matrix = vec![
        vec![],
        vec![17.0],
        vec![21.0,30.0],
        vec![31.0,34.0,28.0],
        vec![23.0,21.0,39.0,43.0],
    ];
    let distance_function = |i:usize,j:usize|distance_matrix[i][j];
    let num_nodes = distance_matrix.len();
    fn assert_children(dendrogram:&Dendrogram, distance:f64, num_children:usize) -> &Vec<Dendrogram> {
        match dendrogram {
            Dendrogram::Leaf(_) => panic!("wasn't expecting a leaf"),
            Dendrogram::Branch(branch) => {
                let delta = branch.distance-distance;
                if delta.abs()>0.0001 {
                    panic!("Distance was {} expecting {}",branch.distance,distance);
                }
                assert_eq!(num_children,branch.children.len());
                &branch.children
            }
        }
    }
    fn assert_leaf(dendrogram:&Dendrogram,index:usize) {
        match dendrogram {
            Dendrogram::Leaf(id) => assert_eq!(index,*id),
            Dendrogram::Branch(_) => panic!("Not expecting a branch"),
        }
    }

    // test efficient simple linkage - not ordered by original order.
    let simple_linkage = Dendrogram::compute_single_linkage(distance_function,num_nodes);
    assert_eq!("[28: 3 [21: [17: 0 1] 2 4]]",simple_linkage.to_string());
    let children28 = assert_children(&simple_linkage,28.0,2);
    let children21 = assert_children(&children28[1],21.0,3);
    let children17 = assert_children(&children21[0],17.0,2);
    assert_leaf(&children17[0],0);
    assert_leaf(&children17[1],1);
    assert_leaf(&children21[1],2);
    assert_leaf(&children21[2],4);
    assert_leaf(&children28[0],3);

    // test inefficient simple linkage
    let simple_linkage = Dendrogram::compute_single_linkage_slow(distance_function,num_nodes);
    assert_eq!("[28: [21: [17: 0 1] 2 4] 3]",simple_linkage.to_string());
    let children28 = assert_children(&simple_linkage,28.0,2);
    let children21 = assert_children(&children28[0],21.0,3);
    let children17 = assert_children(&children21[0],17.0,2);
    assert_leaf(&children17[0],0);
    assert_leaf(&children17[1],1);
    assert_leaf(&children21[1],2);
    assert_leaf(&children21[2],4);
    assert_leaf(&children28[1],3);

    // test complete linkage

    //let complete_linkage = Dendrogram::compute_complete_linkage(distance_function,num_nodes);
    //assert_eq!("[43: [23: [17: 0 1] 4] [28: 2 3]]",complete_linkage.to_string());

    let complete_linkage = Dendrogram::compute_complete_linkage_slow(distance_function,num_nodes);
    assert_eq!("[43: [23: [17: 0 1] 4] [28: 2 3]]",complete_linkage.to_string());

    let mean_linkage = Dendrogram::compute_mean_linkage_slow(distance_function,num_nodes);
    assert_eq!("[35: [22: [17: 0 1] 4] [28: 2 3]]",mean_linkage.to_string());

    let weighted_mean_linkage = Dendrogram::compute_weighted_mean_linkage_slow(distance_function,num_nodes);
    assert_eq!("[33: [22: [17: 0 1] 4] [28: 2 3]]",weighted_mean_linkage.to_string());



}