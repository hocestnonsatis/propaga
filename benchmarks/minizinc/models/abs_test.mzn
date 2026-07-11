int: n = 5;
array[1..n] of var -n..n: xs;
constraint forall(i in 1..n)(xs[i] = abs(xs[i] - 1));
solve satisfy;
