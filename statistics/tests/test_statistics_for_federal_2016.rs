// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use federal::FederalRulesUsed2016;
use federal::parse::get_federal_data_loader_2016;
use statistics::correlations::{CorrelationDendrogramsAndSVD, CorrelationOptions, SquareMatrix};
use stv::errors_btl::ObviousErrorsInBTLVotes;
use stv::find_vote::{FindMyVoteQuery, FindMyVoteResult};
use statistics::intent_table::{IntentTable, IntentTableOptions};
use statistics::mean_preference::{MeanPreferenceByCandidate, MeanPreferences};
use statistics::simple_statistics::SimpleStatistics;
use statistics::who_got_votes::WhoGotVotes;
use stv::parse_util::{FileFinder, RawDataSource};

#[test]
fn test_tasmanian_statistics_using_just_formal_votes() {
    // compare against values computed in the old scala implementation

    let loader = get_federal_data_loader_2016(&FileFinder::find_ec_data_repository());
    let data = loader.load_cached_data("TAS").unwrap();
    let _transcript = data.distribute_preferences::<FederalRulesUsed2016>();

    // Test simple statistics.
    let simple_statistics = SimpleStatistics::new(&data);
    assert_eq!(3767,simple_statistics.num_satl);
    assert_eq!(243774,simple_statistics.num_atl);
    assert_eq!(95385,simple_statistics.num_btl);
    assert_eq!(184249,simple_statistics.num_unique_atl);
    assert_eq!(89611,simple_statistics.num_unique_btl);
    assert_eq!(339159,simple_statistics.num_formal);
    assert_eq!(0,simple_statistics.num_informal);
    assert_eq!(58,simple_statistics.num_candidates);
    assert_eq!(false,simple_statistics.uses_group_voting_tickets);

    // test party/candidate statistics.
    let who_got_votes = WhoGotVotes::compute(&data);
    assert_eq!(5521,who_got_votes.parties[0].first_atl.0);
    assert_eq!(1171,who_got_votes.parties[0].first_btl.0);
    assert_eq!(98814,who_got_votes.parties[0].mention_atl.0);
    assert_eq!(21596,who_got_votes.parties[0].mention_btl.0);
    assert_eq!(83366,who_got_votes.parties[1].first_atl.0);
    assert_eq!(30569,who_got_votes.parties[1].first_btl.0);
    assert_eq!(181477,who_got_votes.parties[1].mention_atl.0);
    assert_eq!(72729,who_got_votes.parties[1].mention_btl.0);

    // test correlations
    fn expect_similar(actual:f64,expected:f64) {
        let diff = actual-expected;
        if diff.abs() > 0.00001 {
            panic!("actual {} expecting {}",actual,expected);
        }
    }
    fn expect(d:&SquareMatrix,i:usize,j:usize,expected:f64) {
        let actual = d.matrix[i][j];
        let diff = actual-expected;
        if diff.abs() > 0.00001 {
            panic!("d[{}][{}] was {} expecting {}",i,j,actual,expected);
        }
    }

    let d = SquareMatrix::compute_correlation_matrix(&data,&CorrelationOptions{
        want_candidates: true,
        use_atl: false,
        use_btl: true,
        subtract_mean: false
    }).to_distance_matrix();
    expect(&d,0,0,0.0);
    expect(&d,0,1,0.01303692991283878);
    expect(&d,0,2,0.28237125468781576);
    expect(&d,0,3,0.28085238514832345);
    expect(&d,0,4,0.282808638380336);
    expect(&d,0,5,0.27679031015779376);
    expect(&d,0,6,0.24561964062097885);
    expect(&d,0,7,0.3907734603844969);
    expect(&d,1,6,0.23348007817716465);

    let d = SquareMatrix::compute_correlation_matrix(&data,&CorrelationOptions{
        want_candidates: false,
        use_atl: true,
        use_btl: true,
        subtract_mean: true,
    }).to_distance_matrix();
    expect(&d,0,0,0.0);
    expect(&d,0,1,1.0743019530531643);
    expect(&d,0,2,1.2046336097190269);
    expect(&d,0,3,0.6172302080989418);
    expect(&d,0,4,1.150497127927257);
    expect(&d,0,5,0.9229782151705949);
    expect(&d,0,6,0.9741901528078853);
    expect(&d,0,7,1.069077809853819);
    expect(&d,1,6,1.0468502848088126);

    let dd = CorrelationDendrogramsAndSVD::new(d).unwrap();
    //println!("{:?}",dd);
    assert_eq!(dd.dendrogram_single.to_string_decimals(4),"[0.8935: [0.7986: 6 [0.7345: [0.7319: 8 9] 12]] [0.8624: 4 [0.8535: [0.6740: 15 18] [0.8356: 7 [0.8349: [0.8343: 10 [0.7905: 13 [0.7772: [0.6172: 0 3] [0.6172: 5 19]]]] [0.8208: [0.7619: 1 [0.7499: 2 [0.7491: 14 [0.7174: 16 [0.6964: 11 [0.5921: 17 20]]]]]] 21]]]]]]");
    assert_eq!(dd.dendrogram_complete.to_string_decimals(4),"[1.3598: [1.3016: [0.9242: [0.6172: 0 3] [0.6172: 5 19]] [1.1754: [0.9383: 6 [0.8440: [0.7319: 8 9] 12]] [1.0816: [0.8651: 7 [0.8231: [0.7350: 11 [0.5921: 17 20]] 16]] [0.9900: [0.9498: [0.8343: 10 13] [0.8208: 14 21]] [0.6740: 15 18]]]]] [1.0593: [0.7619: 1 2] 4]]");
    assert_eq!(dd.dendrogram_mean.to_string_decimals(4),"[1.1362: [1.0651: [0.8589: [0.6172: 0 3] [0.6172: 5 19]] [1.0135: [0.9011: 6 [0.7892: [0.7319: 8 9] 12]] [0.9644: [0.9325: [0.8887: [0.8490: 7 [0.7826: [0.7157: 11 [0.5921: 17 20]] 16]] [0.8208: 14 21]] [0.8343: 10 13]] [0.6740: 15 18]]]] [0.9608: [0.7619: 1 2] 4]]");
    println!("S={}",dd.svd.singular_values);
    println!("U={}",dd.svd.u.as_ref().unwrap());
    println!("V={}",dd.svd.v_t.as_ref().unwrap());
    expect_similar(dd.svd.singular_values[0], 3.1258300127600345);
    expect_similar(dd.svd.singular_values[1], 2.231716850074084);
    expect_similar(dd.svd.singular_values[2], 1.9879444894231315);
    expect_similar(dd.svd.singular_values[3], 1.4155257960264707);
    expect_similar(dd.svd.u.as_ref().unwrap()[0].abs(),0.22865175155120554);
    expect_similar(dd.svd.u.as_ref().unwrap()[1].abs(),0.10660239874934234);
    expect_similar(dd.svd.u.as_ref().unwrap()[2].abs(),0.33155040171072736);
    expect_similar(dd.svd.v_t.as_ref().unwrap()[0].abs(),0.22865175155120546);
    expect_similar(dd.svd.v_t.as_ref().unwrap()[1].abs(),0.03263840268676327);
    expect_similar(dd.svd.v_t.as_ref().unwrap()[2].abs(),0.12276714431820211);
    // test intent tables

    let intent = IntentTable::compute(&data,&IntentTableOptions{
        first_pref_by_groups: false,
        who_is_groups: false,
        use_atl: true,
        use_btl: true,
        who: vec![2,8]
    });

    assert_eq!(intent.table[0][0].0,3277);
    assert_eq!(intent.table[0][1].0,950);
    assert_eq!(intent.table[0][2].0,2290);
    assert_eq!(intent.table[1][0].0,43);
    assert_eq!(intent.table[1][1].0,25);
    assert_eq!(intent.table[1][2].0,107);

    let intent = IntentTable::compute(&data,&IntentTableOptions{
        first_pref_by_groups: true,
        who_is_groups: true,
        use_atl: true,
        use_btl: true,
        who: vec![1,2,5]
    });

    assert_eq!(intent.table[0][0].0,2767); // Family First -> ALP
    assert_eq!(intent.table[0][1].0,772);  // Family First -> Greens
    assert_eq!(intent.table[0][2].0,2069); // Family First -> Labot
    assert_eq!(intent.table[0][3].0,1084); // Family First -> None of the above
    assert_eq!(intent.table[1][0].0,113935); // ALP -> ALP
    assert_eq!(intent.table[1][1].0,0);
    assert_eq!(intent.table[1][2].0,0);
    assert_eq!(intent.table[1][3].0,0);

    // test mean preferences

    let mean = MeanPreferences::compute(&data);
    fn expect_mean(what:&MeanPreferenceByCandidate,atl:usize,btl:usize,prefs:Vec<f64>) {
        assert_eq!(atl,what.num_atl.0);
        assert_eq!(btl,what.num_btl.0);
        for i in 0..prefs.len() {
            let expected = prefs[i];
            let actual = what.mean_preference[i];
            let diff = expected-actual;
            if diff.abs() > 0.00001 {
                panic!("mean_preference[{}] was {} expecting {}",i,actual,expected);
            }
        }
    }
    expect_mean(&mean.all,243774,95385,vec![29.409148806312082, 29.894508180528895, 15.563082507024728, 16.081063748861155]);
    expect_mean(&mean.all_by_first_preference[0],5521,996,vec![ 1.0, 3.22847936166948, 19.9566518336658, 20.486036519871107, 21.351388675771062, 22.010817860978978, 22.5450360595366, 22.91990179530459, 28.04764462175848, 28.700475678993403]);
    expect_mean(&mean.all_by_first_preference[1],0,175,vec![16.665714285714287, 1.0, 29.982857142857142, 27.414285714285715, 29.96, 30.92, 29.63714285714286, 29.18, 30.92, 30.96]);
    expect_mean(&mean.btl,0,95385,vec![32.42563296115742, 33.11546888923835, 19.601446768359807, 19.540651045761912, 19.700587094406877, 20.06886302877811, 21.70380562981601, 14.858106620537821, 22.21398018556377]);
    expect_mean(&mean.btl_by_first_preference[0],0,996,vec![ 1.0,10.038152610441767, 27.183232931726906, 27.296686746987948, 29.608433734939755, 30.572791164658632, 30.71787148594377, 29.82028112449799, 30.989959839357425, 33.084839357429715]);
    expect_mean(&mean.btl_by_first_preference[1],0,175,vec![16.665714285714287, 1.0, 29.982857142857142, 27.414285714285715, 29.96, 30.92, 29.63714285714286, 29.18, 30.92, 30.96]);

}

#[test]
fn test_obvious_btl_errors() {
    // compare against values computed in the old scala implementation

    let loader = get_federal_data_loader_2016(&FileFinder::find_ec_data_repository());
    let errors = ObviousErrorsInBTLVotes::compute(&loader,"TAS").unwrap();
    println!("{:?}",errors);
    assert_eq!(2643,errors.repeated[0]);
    assert_eq!(1547,errors.repeated[1]);
    assert_eq!(584,errors.repeated[2]);
    assert_eq!(328,errors.repeated[3]);
    assert_eq!(324,errors.repeated[4]);
    assert_eq!(573,errors.repeated_papers[0].0);
    assert_eq!(385,errors.repeated_papers[1].0);
    assert_eq!(303,errors.repeated_papers[2].0);
    assert_eq!(231,errors.repeated_papers[3].0);
    // numbers below here have a different meaning to numbers from the old Scala interpretation so are not verified by it.
    assert_eq!(233,errors.missing[0].0);
    assert_eq!(36,errors.missing[1].0);
    assert_eq!(47,errors.missing[2].0);
    assert_eq!(46,errors.missing[3].0);
    assert_eq!(242535,errors.ok_up_to[0].0);
    assert_eq!(570,errors.ok_up_to[1].0);
    assert_eq!(255,errors.ok_up_to[2].0);
    assert_eq!(171,errors.ok_up_to[3].0);
    assert_eq!(107,errors.ok_up_to[4].0);
    assert_eq!(136,errors.ok_up_to[5].0);
    assert_eq!(2602,errors.ok_up_to[6].0);
}


#[test]
fn test_find_vote() {
    // compare against values computed in the old scala implementation

    let loader = get_federal_data_loader_2016(&FileFinder::find_ec_data_repository());
    let found = FindMyVoteResult::compute(&loader, "TAS",&FindMyVoteQuery{ query: "9,10,21,22,23,24,25,26,54,55,56,7,8,19,20,1,2,3,4,5,6,51,52,53,49,50,47,48,57,58,29,30,13,14,44,45,46,33,34,42,43,11,12,31,32,27,28,15,16,17,18,35,36,37,38,39,40,41".to_string(), blank_matches_anything: false }).unwrap();
    // Test at http://192.168.2.20:8095/tools/federal/2016/TAS/PrepareVote.html
    // This is the first listed vote from AEC : 9,10,21,22,23,24,25,26,54,55,56,7,8,19,20,1,2,3,4,5,6,51,52,53,49,50,47,48,57,58,29,30,13,14,44,45,46,33,34,42,43,11,12,31,32,27,28,15,16,17,18,35,36,37,38,39,40,41
    assert_eq!(3,found.best.len());
    assert_eq!(58,found.best[0].score);
    assert_eq!(0,found.best[0].truncated);
    assert_eq!(1,found.best[0].hits.len());
    assert_eq!("9,10,21,22,23,24,25,26,54,55,56,7,8,19,20,1,2,3,4,5,6,51,52,53,49,50,47,48,57,58,29,30,13,14,44,45,46,33,34,42,43,11,12,31,32,27,28,15,16,17,18,35,36,37,38,39,40,41",&found.best[0].hits[0].votes);
    assert_eq!("Bass",found.best[0].hits[0].metadata["Electorate"]);
    assert_eq!("Branxholm",found.best[0].hits[0].metadata["Collection Point"]);
    assert_eq!(17,found.best[1].score);
    assert_eq!(0,found.best[1].truncated);
    assert_eq!(1,found.best[1].hits.len());
    assert_eq!("17,18,21,22,23,24,25,26,55,56,54,19,20,27,28,1,3,2,4,5,6,53,52,51,49,50,47,48,45,46,29,30,31,32,7,8,9,41,42,43,44,57,58,10,11,33,34,35,36,37,38,39,40,12,13,14,15,16",&found.best[1].hits[0].votes);
    assert_eq!(16,found.best[2].score);
    assert_eq!(0,found.best[2].truncated);
    assert_eq!(3,found.best[2].hits.len());


    println!("{:?}", found);
}