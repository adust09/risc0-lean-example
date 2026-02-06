partial def fibonacci (n : Nat) : Nat :=
  if n ≤ 1 then n
  else
    let rec loop (i curr prev : Nat) : Nat :=
      if i > n then curr
      else loop (i + 1) (prev + curr) curr
    loop 2 1 0
