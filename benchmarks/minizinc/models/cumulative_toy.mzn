array[1..2] of var 0..10: s;
array[1..2] of var 1..5: d;
array[1..2] of var 0..15: e;
constraint cumulative(s, d, e, 1);
solve satisfy;
