// tyrian's generated sources, as its Mill build has its own generators write them (tyrian-tags/package.mill's
// `_customGeneratedSources`, tyrian-htmx/package.mill's `_customHtmxGeneratedSources`), run by teq as the
// overlay's generator commands (teq.mill.json): the build's generator files (tyrian-tags/*.mill,
// tyrian-htmx/HtmxAttributes.mill) are this program's sources as they are, beside os.scala for the part of
// os-lib they call.
//
// teq interp generate.scala os.scala <the build's generator files> -- tags|htmx <output>
@main def generate(args: String*): Unit =
  if args.length != 2 then
    System.err.println("usage: generate.scala os.scala <generator files> -- tags|htmx <output>")
    sys.exit(2)
  // The directory exists, as Mill's `Task.dest` does for the build's own run.
  val dest = os.Path(java.nio.file.Files.createDirectories(java.nio.file.Paths.get(args(1)).toAbsolutePath.normalize))
  args(0) match
    case "tags" =>
      build.`tyrian-tags`.AttributeGen.gen("tyrian", dest)
      build.`tyrian-tags`.HtmlEntityGen.gen("tyrian", dest)
      build.`tyrian-tags`.CSSGen.gen("CSS", "tyrian", dest)
      build.`tyrian-tags`.TagGen.gen("tyrian", dest)
    case "htmx" =>
      build.`tyrian-htmx`.HtmxAttributes.gen("tyrian.htmx", dest)(build.`tyrian-htmx`.HtmxAttributes.htmxAttrsList)
    case other =>
      System.err.println(s"no generator $other")
      sys.exit(2)
