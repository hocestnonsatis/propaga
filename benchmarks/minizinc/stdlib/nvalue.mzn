array[1..3] of var 1..3: xs;
var 1..3: n;
constraint nvalue(n, xs);
solve satisfy;
