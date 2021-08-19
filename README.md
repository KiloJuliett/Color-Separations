# Color Separations

This repository contains the source code for a command line application for
performing arbitrary color separation. Given any number of input colors, this
application will attempt to simulate what would happen if those input colors
were used as the primary colors in a process color printing job, generating
3D lookup tables (3D LUTs) so that the results can be used in your favorite
image or video editing software.

This is ostensibly an artist's tool. For everyone who might it useful---y'know,
like, both of them---I hope you find this little utility to your liking.

## Basic Example

Suppose you have the following image:

![Source image of a blossoming cherry tree](/examples/cherry_source.jpg)

And you wanted to create an effect that would simulate what this image would
look like if printed using a two-color printing process with colors of your
choice. The colors you would like to use, expressed as sRGB coordinates, are
`#026037` (a cool forest green) and `#7c399f` (a dark lavender).

The following command:

```
    separations -p sRGB -o cherry.cube -c 2 96 55 -c 124 57 159
```

Will produce five 3D LUTs that will allow you to create this effect in any image
or video editor that accepts 3D LUTs for performing color lookup filters. When
applied to the original image, these 3D LUTs will have the following results:

  - **cherry.cube:**
    ![Image above after having cherry.cube applied to it](/examples/cherry_result.jpg)
  - **cherry_0.cube:**
    ![Image above after having cherry_0.cube applied to it](/examples/cherry_result_0.jpg)
  - **cherry_0m.cube:**
    ![Image above after having cherry_0m.cube applied to it](/examples/cherry_result_0m.jpg)
  - **cherry_1.cube:**
    ![Image above after having cherry_1.cube applied to it](/examples/cherry_result_1.jpg)
  - **cherry_1m.cube:**
    ![Image above after having cherry_1m.cube applied to it](/examples/cherry_result_1m.jpg)
