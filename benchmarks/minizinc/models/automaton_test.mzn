array[1..3] of var 1..2: x;
constraint regular(x, 2, 3, [1, 2, 0, 1, 2, 0], 1, 2);
solve satisfy;
