// A directory with a script beside a program's main. scala-cli compiles both and runs the one main that is no
// script's (Build.retainedMainClass: inferredMainClass, telling of the script's); its run's output is the .expected's.
// teq's walk takes the script under teq interp, teq compiler check and a check session; teq compiler build and a
// build session take none, and with it would refuse the two entry points, as teq compiler check does today.
object Main { def main(args: Array[String]): Unit = println("main") }
