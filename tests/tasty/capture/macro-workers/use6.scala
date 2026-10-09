object UseMacroWorkers6:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
