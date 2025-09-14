# ltrender

powerful rendering engine for terminal tools and games.


## TODO:
- documentation
- more examples
- allow for two methods of keypress handling (at best both should be possible at the same time)
1. getting a list of all the changes and the user can filter out themselves what to use  
this is meant for a frame based usecase
2. setting up a sender and receiver. the InputHandler can then send changes to the Code directly,
this would be meant for applications, that need to be updated instantly

## Improve:
- add more testing

## Remember:
- remember to make the renderers interval expansion happen, after the diffing has been done, so intervals, that have not change aren diffed