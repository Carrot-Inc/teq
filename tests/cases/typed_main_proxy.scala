// A typed @main method runs through dotty's MainProxies proxy, which parses the command line into its parameters
// (CommandLineParser): run without arguments, the missing first one is the error it shows.
@main def typed(n: Int, b: Boolean, rest: String*): Unit = println(s"$n $b ${rest.toList}")
