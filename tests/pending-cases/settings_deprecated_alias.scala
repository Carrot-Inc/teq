//> using options -Xfatal-warnings
// scalac's settings warn of a deprecated alias (Settings.scala:173-198, ScalaSettings.scala:167), a configuration
// warning (CliCommand.checkUsage) that -Werror then fails (Reporter.werror); teq is to warn of it too.
@main def run(): Unit = println(1)
