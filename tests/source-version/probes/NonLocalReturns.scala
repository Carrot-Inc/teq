object P { def f(): Int = { List(1).foreach(x => return x); 0 } }
