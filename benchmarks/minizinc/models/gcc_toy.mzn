array[1..4] of var 1..3: xs;
array[1..3] of int: cover = [1, 2, 3];
constraint global_cardinality(xs, cover);
solve satisfy;
