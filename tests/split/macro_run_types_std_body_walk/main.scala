package m

// `macro_run_types_std_body` with its macro's value written out: here a walk types `Vector.empty` first. The two
// programs' standard library modules are the same (tests/split.sh, macro-std-body).
@main def run(): Unit = println(1 + Vector(1, 2).size + Vector.empty[Int].size)
