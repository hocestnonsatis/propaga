array[1..2] of var 0..10: s;
array[1..2] of var 1..5: d;
constraint disjunctive(s, d);
solve satisfy;
