package m

// A method of the standard library that a macro's run reaches before any walk types it is typed for the run as the
// walk types it: its object's own member is selected on `this`. So the standard library's module is the same with
// one worker, where the run types the body first, and with several, where a worker's walk does.
@main def run(): Unit = println(M.n + Vector(1, 2).size + Vector.empty[Int].size)
