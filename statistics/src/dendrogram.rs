// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use serde::{Serialize,Deserialize};

/// a node in a dendrogram.
type NodeIndex=usize;
type LinkageDistance = f64;


struct PointerRepresentation {
    /// λ(i) is the lowest level at which i is no longer the last object in its cluster
    λ : Vec<LinkageDistance>,
    /// π(i) is the last object in the cluster which it then join
    π:Vec<usize>,
    //  Sum_{i<j} d(i,j).
    // sum_of_all_distances : LinkageDistance,
}

impl PointerRepresentation {

    /// Compute a dendrogram using single link, using the method of R.Sibson, "SLINK: An optimally efficient algorithm for the single-link cluster method"
    /// Note that there are some changes since the arrays are 0 based. In particular, the indices are all 1 less than the paper, and the values
    /// of π, being indices, are also one less than the paper.
    pub fn s_link_dendrogram_create_pointer_representation<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> PointerRepresentation {
        let mut m = vec![0.0;num_nodes];
        let mut λ = vec![0.0;num_nodes];
        let mut π : Vec<NodeIndex> = vec![0;num_nodes];
        //let mut sum_of_all_distances : LinkageDistance = 0.0;
        for n in 0..num_nodes {
            // Step 1
            π[n]=n;
            λ[n]=f64::INFINITY;
            // Step 2
            for i in 0..n {
                m[i] = distance(n,i);
                //sum_of_all_distances+=m[i];
            }
            // Step 3
            for i in 0..n {
                if λ[i]>=m[i] {
                    m[π[i]]=LinkageDistance::min(m[π[i]],λ[i]);
                    λ[i]=m[i];
                    π[i]=n
                } else {
                    m[π[i]]=LinkageDistance::min(m[π[i]],m[i]);
                }
            }
            // Step 4
            for i in 0..n {
                if λ[i]>=λ[π[i]] { π[i]=n; }
            }
        }
        PointerRepresentation{λ,π}
    }

    /// Compute a dendrogram using complete linkage, using the method of D. Defays, "An efficient algorithm for a complete link method"
    /// Note that there are some changes since the arrays are 0 based. In particular, the indices are all 1 less than the paper, and the values
    /// of π, being indices, are also one less than the paper.
    ///
    /// NOTE THIS IS BUGGY DO NOT USE
    pub fn c_link_dendrogram_create_pointer_representation<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> PointerRepresentation {
        let mut m = vec![0.0;num_nodes];
        let mut λ = vec![0.0;num_nodes];
        let mut π : Vec<NodeIndex> = vec![0;num_nodes];
        //let mut sum_of_all_distances : LinkageDistance = 0.0;
        π[0]=0;
        λ[0]=f64::INFINITY;
        for n in 1..num_nodes {
            // Step 1
            π[n]=n;
            λ[n]=f64::INFINITY;
            // Step 2
            for i in 0..n {
                m[i]=distance(n,i);
                //sum_of_all_distances+=m[i];
            }
            // Step 3
            for i in 0..n {
                if λ[i]<m[i] {
                    m[π[i]] = LinkageDistance::max(m[π[i]], m[i]);
                    m[i] = f64::INFINITY;
                }
            }
            // Step 4
            let mut a=n-1;
            // Step 5
            for i in 0..n {
                let index = n-1-i;  // could just do loop descending
                if λ[index]>=m[π[index]] {
                    if m[index]<m[a] { a=index }
                } else {
                    m[index]=f64::INFINITY;
                }
            }
            // Step 6
            let mut b=π[a];
            let mut c=λ[a];
            π[a]=n;
            λ[a]=m[a];
            // Step 7
            while a<n-1 && b<n-1 {
                let d=π[b];
                let e=λ[b];
                π[b]=n;
                λ[b]=c;
                b=d;
                c=e;            }
            if a<n-1 && b==n-1 {
                π[b]=n;
                λ[b]=c;
            }
            // Step 8
            for i in 0..n {
                if π[π[i]]==n && λ[i]>=λ[π[i]] { π[i]=n; }
            }
        }
        println!("λ={:?}  π={:?}",λ,π);
        PointerRepresentation{λ,π}
    }

    pub fn to_dendrogram(&self) -> Dendrogram {
        let merge_order : Vec<NodeIndex> = {
            let mut res : Vec<NodeIndex> = (0..self.λ.len()-1).collect();
            res.sort_by(|&x,&y|{
                let d = self.λ[x]-self.λ[y];
                if d>0.0 { Ordering::Greater }
                else if d<0.0 { Ordering::Less }
                else {self.π[x].cmp(&self.π[y])}
            });
            res
        };
        let mut nodes : Vec<Dendrogram> = (0..self.λ.len()).map(|n|Dendrogram::Leaf(n)).collect();
        let mut lastπ = usize::MAX;
        let mut lastλ = LinkageDistance::NAN;
        let mut buildup : Vec<Dendrogram> = vec![];
        for m in merge_order {
            if lastπ != self.π[m] || lastλ != self.λ[m] {
                if !buildup.is_empty() {
                    buildup.push(std::mem::replace(&mut nodes[lastπ],Dendrogram::Leaf(NodeIndex::MAX))); // replace with a dummy value.
                    nodes[lastπ]=Dendrogram::Branch(Box::new(DendrogramBranch{children:buildup,distance:lastλ}));
                    buildup=vec![];
                }
                lastπ = self.π[m];
                lastλ = self.λ[m];
            }
            buildup.push(std::mem::replace(&mut nodes[m],Dendrogram::Leaf(NodeIndex::MAX))); // replace with a dummy value.
        }
        { // exactly the same code as a couple of lines above :-(
            buildup.push(std::mem::replace(&mut nodes[lastπ],Dendrogram::Leaf(NodeIndex::MAX))); // replace with a dummy value.
            nodes[lastπ]=Dendrogram::Branch(Box::new(DendrogramBranch{children:buildup,distance:lastλ}));
            // buildup=vec![];
        }
        nodes.pop().unwrap()
    }
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum Dendrogram {
    Leaf(NodeIndex),
    Branch(Box<DendrogramBranch>)
}

impl Display for Dendrogram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Dendrogram::Leaf(id) => write!(f,"{}",id),
            Dendrogram::Branch(branch) => {
                write!(f,"[{}:",branch.distance)?;
                for c in &branch.children {
                    write!(f," {}",c)?;
                }
                write!(f,"]")
            }
        }
    }
}

impl Dendrogram {
    /// convert to a string with a specific precision (number of decimal digits).
    pub fn to_string_decimals(&self,precision:usize) -> String {
        match self {
            Dendrogram::Leaf(id) => format!("{}",id),
            Dendrogram::Branch(branch) => {
                let mut res = format!("[{:.*}:",precision,branch.distance);
                for c in &branch.children {
                    res+=" ";
                    res+=&c.to_string_decimals(precision);
                }
                res+="]";
                res
            }
        }
    }
}
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct DendrogramBranch {
    pub children : Vec<Dendrogram>,
    pub distance : LinkageDistance,
}

impl Dendrogram {
    /// Efficiently create a single linkage dendrogram. O(n^2).
    /// Only call the distance function d(i,j) with i>j.
    pub fn compute_single_linkage<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> Dendrogram {
        PointerRepresentation::s_link_dendrogram_create_pointer_representation(distance,num_nodes).to_dendrogram()
    }
    /// Efficiently create a complete linkage dendrogram. O(n^2).
    /// Only call the distance function d(i,j) with i>j.
    ///
    /// NOTE THIS IS BUGGY DO NOT USE
    pub fn compute_complete_linkage<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> Dendrogram {
        PointerRepresentation::c_link_dendrogram_create_pointer_representation(distance,num_nodes).to_dendrogram()
    }
    /// Inefficiently create a single linkage dendrogram. O(n^3).
    /// Only call the distance function d(i,j) with i>j.
    pub fn compute_single_linkage_slow<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> Dendrogram {
        Dendrogram::slow_compute_dendrogram(distance,num_nodes,|_,d1,_,d2|d1.min(d2))
    }
    /// Inefficiently create a complete linkage dendrogram. O(n^3).
    /// Only call the distance function d(i,j) with i>j.
    pub fn compute_complete_linkage_slow<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> Dendrogram {
        Dendrogram::slow_compute_dendrogram(distance,num_nodes,|_,d1,_,d2|d1.max(d2))
    }
    /// Inefficiently create a mean linkage dendrogram. O(n^3).
    /// Only call the distance function d(i,j) with i>j.
    pub fn compute_mean_linkage_slow<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> Dendrogram {
        Dendrogram::slow_compute_dendrogram(distance,num_nodes,|_,d1,_,d2|0.5*(d1+d2))
    }
    /// Inefficiently create a weighted mean linkage dendrogram. O(n^3).
    /// Only call the distance function d(i,j) with i>j.
    pub fn compute_weighted_mean_linkage_slow<F:Fn(NodeIndex,NodeIndex)->LinkageDistance>(distance:F, num_nodes:usize) -> Dendrogram {
        Dendrogram::slow_compute_dendrogram(distance,num_nodes,|w1,d1,w2,d2|(d1*w1 as f64+d2*w2 as f64)/(w1+w2) as f64)
    }


    /// a slow, obvious way to compute a dendrogram. Find the closest two, merge, repeat. Horrendously inefficient, O(n^3). But the number of candidates is rarely over a thousand...
    /// Fionn Murtagh has an O(n^2) algorithm I could conceivably implement.
    /// Only call the distance function d(i,j) with i>j.
    /// The linkage function takes the size of node1, the distance to node 1, the size of node 2, the distance to node 2, and returns the distance to the node comprised of nodes 1 and 2.
    fn slow_compute_dendrogram<F:Fn(NodeIndex,NodeIndex)->LinkageDistance,L:Fn(usize,LinkageDistance,usize,LinkageDistance)->LinkageDistance>(distance:F, num_nodes:usize,update_linkage:L) -> Dendrogram {
        let mut nodes : Vec<Dendrogram> = (0..num_nodes).map(|n|Dendrogram::Leaf(n)).collect();
        let mut node_size : Vec<usize> = vec![1;num_nodes];
        // nodes[i] contains node_size[i] leaves.
        let mut d : Vec<Vec<LinkageDistance>> = (0..num_nodes).map(|i|(0..i).map(|j|distance(i,j)).collect::<Vec<_>>()).collect();
        for n in (2..=num_nodes).rev() {
            assert_eq!(n,nodes.len());
            // find the smallest distance
            let mut larger_index = usize::MAX;
            let mut smaller_index = usize::MAX;
            let mut smallest_distance = f64::INFINITY;
            for i in 1..n {
                for j in 0..i {
                    if d[i][j]<smallest_distance {
                        smallest_distance=d[i][j];
                        smaller_index=j;
                        larger_index=i;
                    }
                }
            }
            // we will now merge smaller_index and larger_index together, making a new node in smaller_index, and shuffling down nodes larger than larger_index by 1.
            let old_smaller_node = std::mem::replace(&mut nodes[smaller_index],Dendrogram::Leaf(NodeIndex::MAX));
            let old_larger_node = nodes.remove(larger_index);
            let mut children = vec![];
            let mut add_node = |n:Dendrogram| match n {
                Dendrogram::Branch(mut branch) if (smallest_distance-branch.distance)<0.00000001*smallest_distance => { children.append(&mut branch.children)}
                _ => { children.push(n) }
            };
            add_node(old_smaller_node);
            add_node(old_larger_node);
            nodes[smaller_index] = Dendrogram::Branch(Box::new(DendrogramBranch{ children, distance: smallest_distance }));
            let size_old1 = node_size[smaller_index];
            let size_old2 = node_size[larger_index];
            node_size[smaller_index]+=size_old2;
            node_size.remove(larger_index);
            // update the distance matrix with distances to new nodes.
            for i in 0..n {
                if i!=smaller_index && i!=larger_index {
                    let d_i_old1 = d[i.max(smaller_index)][i.min(smaller_index)];
                    let d_i_old2 = d[i.max(larger_index)][i.min(larger_index)];
                    let d_i_newnode = update_linkage(size_old1,d_i_old1,size_old2,d_i_old2);
                    d[i.max(smaller_index)][i.min(smaller_index)]=d_i_newnode;
                }
            }
            // remove the unneeded distance to larger_index.
            for i in larger_index+1..n {
                d[i].remove(larger_index);
            }
            d.remove(larger_index);
            // note that it would be more efficient to use swap_remove, but that would mess up the triangular shape of d, and it is already O(n^3) by the search
        }
        assert_eq!(1,nodes.len());
        nodes.pop().unwrap()
    }
}
