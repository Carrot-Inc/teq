/* header */ // ordinary comment
//> using dep dev.zio::zio:2.1.25
println("ok")
// The block comment's closing line holds more than blanks, so Scala CLI's extractor takes it for code: the
// directive after it is ignored, with a warning, and wrong-version.sc's version reaches no lock.
