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

1. **Download** the `addons/godot-bevy` folder from our [repository](https://github.com/bytemeadow/godot-bevy)
2. **Create** the `/addons` folder if it is missing. This is where all plugins live inside Godot projects.
3. **Copy** the `godot-bevy` plugin to your project's `/addons` folder.
<details>
<summary>View screenshot</summary>

![Finder structure with addon folder copied](../images/addon-copy.png)

</details>

4. **Open** the project settings and navigate to the Plugins section.
<details>
<summary>View screenshot</summary>

![Godot AssetLib Plugins button](../images/project-settings-menu.png)

</details>

5. **Enable** godot-bevy

<details>
<summary>View screenshot</summary>

![Enabled godot-bevy plugin button](../images/enable-plugin.png)

</details>

## Generate the `godot-bevy` Project

Next we'll create the rust project via our Godot Editor Plugin's > Tools functionality.

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
