# Creating a New Project

In this section we'll create a new Godot project, set up `godot-bevy` in it, create our first Bevy entities, build our first scene using the entity-generated nodes, instantiate that scene from a Bevy system, and control our first node.

## 1. Create a New Godot Project

In this step, we'll create a new Godot project to use for this guide. Make sure you meet the [system requirements](./index.md#system-requirements), then follow these steps:

1. **Open** Godot
2. Click **Create**

<details>
<summary>View screenshot</summary>

![Godot screenshot showing Create button](../images/create-button.png)

</details>

3. Fill in your project details and **Create** your project

<details>
<summary>View screenshot</summary>

![Godot create project modal](../images/create-modal.png)

</details>


## 2. Install the Godot Editor Plugin

Next we'll install `godot-bevy`'s Godot Editor Plugin. The plugin has a project creation wizard which will make things a ton easier for us!

1. **Download** a zip of the [release](https://github.com/bytemeadow/godot-bevy/releases) associated with the version.

<details>
<summary>View screenshot</summary>

![Release page asset downloads](../images/release-zips.png)

</details>

2. **Extract and Copy** the `godot-bevy` plugin to your project's `/addons` folder from the zipped release you downloaded in the previous step. You can find the plugin at `godot-bevy-<x.y.z version>/addons/godot-bevy`.
<details>
<summary>View screenshot</summary>

![Finder structure with addon folder copied](../images/addon-copy.png)

</details>

3. **Open** Project > Project settings and navigate to the Plugins tab.
<details>
<summary>View screenshot</summary>

![Godot project settings screen](../images/project-settings-menu.png)

</details>

4. **Enable** godot-bevy

<details>
<summary>View screenshot</summary>

![Enabled godot-bevy plugin button](../images/enable-plugin.png)

</details>

## Generate the `godot-bevy` Project

Next we'll create the Bevy rust project via our Godot Editor Plugin's > Tools functionality. This will create the basic Bevy boilerplate code for us as well as the `rust.gdextension` file to link it to Godot.

1. **Open** Project > Tools > Setup `godot-bevy` project

<details>
<summary>View screenshot</summary>

![Setup godot-bevy project in Project > Tools menu](../images/setup-godot-bevy-project.png)

</details>

2. Fill in project details and **Create Project**. This starts a background process to generate the rust code for the bevy project _(will take a moment)_.

<details>
<summary>View screenshot</summary>

![Setup godot-bevy project in Project > Tools menu](../images/create-godot-bevy-project.png)

![Generated Bevy project inside and editor](../images/bevy-project-in-editor.png)

</details>

## Creating a scene

1. **Create** a new  3D scene node and rename it to "Main" by double clicking it. 

<details>
<summary>View screenshot</summary>

![Create a main scene](../images/create-scene-3d.png)

</details>

2. Set this to the project's main scene by opening Project -> Project Settings -> General -> Application -> Run. You can also press `f5` and a prompt will appear, allowing you to select the current scene as the main one.

<details>
<summary>View screenshot</summary>

![Set project a main scene](../images/main-scene-settings.png)
![Select a main scene from f5](../images/select-main-scene.png)

</details>

3. Run the project and check out the output window to see a message from the Bevy application printed every second.
