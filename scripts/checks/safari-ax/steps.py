# What run.py asks of the page page.py makes, in order: (command, target, text, what the answer holds, what the
# page's «status:» line then says, whether the command is one that waits before it answers).
FAST, WAITS = False, True
STEPS = [
    ('type', 'Type a message', 'Günaydın reis!', ['it now says «Günaydın reis!»'], '', FAST),
    ('click', 'Send', '', ['clicked AXButton «Send»'], 'status: sent Günaydın reis!', FAST),
    ('click', 'React to message 1499', '', ['clicked AXButton «React to message 1499»'], 'status: reacted 1499', FAST),
    ('type', 'Search', 'hola', ['it now says «hola»'], 'status: searched hola', FAST),
    ('click', 'Never', '', ['ERROR not clicked, it is disabled'], 'status: searched hola', FAST),
    # a command that finds nothing looks again for two seconds, then names what is close, else what the page has
    ('click', 'zzqx nothing here 7731', '', ['ERROR not found: zzqx nothing here 7731. The page has: button «Chat 0»'], 'status: searched hola', WAITS),
    ('click', 'Sennd the message', '', ['ERROR not found: Sennd the message. Close to it: button «React to message 0», '], 'status: searched hola', WAITS),
    ('type', 'Serch box', 'x', ['ERROR not found: Serch box. The page has: ', 'field «Search» holds «hola»'], 'status: searched hola', WAITS),
    # what can be pressed and typed into, with the words a click and typing take; with words, what holds them
    ('find', '', '', ['button «Chat 0»\nbutton «Chat 1»', '… and ', ' more: give words to see fewer'], '', FAST),
    ('find', 'React to message 1499', '', ['button «React to message 1499»'], '', FAST),
    ('find', 'Search', '', ['field «Search» holds «hola»'], '', FAST),
    ('find', 'Teams meeting', '', ['checkbox «Teams meeting» on'], '', FAST),
    ('find', 'Colour', '', ['list «Colour» holds «Red»'], '', FAST),
    ('find', 'Never', '', ['button «Never» (disabled)'], '', FAST),
    ('find', 'Message number 77', '', ['text «Message number 77 with'], '', FAST),
    ('find', 'qqq no such thing', '', ['ERROR nothing with «qqq no such thing» on this page. The page has: button «Chat 0»'], '', FAST),
    # inside a frame of another origin, where a tab's text is not read
    ('find', 'Inside frame', '', ['button «Inside frame»'], '', FAST),
    ('click', 'Inside frame', '', ['clicked AXButton «'], '', FAST),
    ('find', 'Framed done', '', ['button «Framed done»'], '', FAST),
    # what a page shows a moment later: a click looks again by itself, `wait` for longer
    ('click', 'Start soon', '', ['clicked AXButton «Start soon»'], '', FAST),
    ('click', 'Late thing', '', ['clicked AXButton «Late thing»'], 'status: pressed Late thing', WAITS),
    ('click', 'Start late', '', ['clicked AXButton «Start late»'], '', FAST),
    ('wait', 'Much later thing', '', ['there: button «Much later thing»'], '', WAITS),
    ('click', 'Much later thing', '', ['clicked AXButton «Much later thing»'], 'status: pressed Much later thing', FAST),
    ('wait', 'Send', '', ['there: button «Send»'], '', FAST),
]
