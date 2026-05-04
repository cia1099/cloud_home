# Cloud Home
* Create Rust project
```sh
cargo new <project name>
```
* Create NextJS project
```sh
npx create-next-app@latest <project name>
```

* ## Plan
```
I want you to help me write a spec file for a project I am building. It's called "hookhub". It's a place where cool open source Claude hooks are displayed and browsed. Search on Claude hooks and write an initial spec for this. Remember it's an MVP ATM and we need only the functionality of displaying the hooks. Hooks are found in GitHub repositories, they have name, category, description and link to repo. The main page should display the hooks in a grid-like view.
```
我希望你能帮我为我正在开发的项目编写一个规范文件。这项目是家里云端，目标是代替Google drive，实现一个像Google drive一样的云端存储服务，后端用Rust来管理文件系统，用Rust的Restful API来定义接口来访问文件系统。 我已经准备好了外接硬盘，希望可以程序自动侦测外接硬盘的挂载，将API访问的文件系统都是读写到这个外接硬盘上。然后也删除档案可以像操作系统的回收桶一样，预留1个礼拜的期限，让用户可以把删除的档案，在回收站里找回并取消删除，直到期限到期，再由Rust执行强制删除在这外接硬盘的档案，例如"rm -rf"。然后这个服务要做到每个账户隔离可见的文件在这个硬盘的内容，用户只能看见自己账号上传的文件和资料夹；除非用户分享该文件的链接，让知道链接的人可见和下载。前端稍后再考虑如何来实现，现阶段后端优先，先实现所有必要的API接口，可以管理文件系统和账户区隔可见的文件。