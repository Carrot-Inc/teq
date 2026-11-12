//> using options -nowarn --no-warnings:false -Werror
// Two names of one Boolean setting with conflicting values (Settings.scala:201-210): scalac
// warns of the conflict, a configuration warning that -Werror fails; teq is to warn of it too.
@main def run(): Unit = println(1)
