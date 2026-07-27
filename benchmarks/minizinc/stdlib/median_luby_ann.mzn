var 1..5: x;
constraint x <= 3;
solve :: restart_luby(32) :: int_search([x], activity, indomain_median, complete) satisfy;
