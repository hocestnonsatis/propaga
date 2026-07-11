array[1..9] of var 1..9: xs;
constraint all_different(xs);
constraint sum(xs) = 15;
solve satisfy;
