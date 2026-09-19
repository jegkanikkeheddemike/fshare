import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { api } from "../api";
import { Icon } from "@fluentui/react";
import { getFileTypeIconProps } from "@fluentui/react-file-type-icons";

type DirEntry = {
    relative_name: string,
    is_dir: boolean,
    mime: string | null
}
type DirContent = {
    entries: DirEntry[]
}

export const BrowsePage = () => {
    const { "*": rawPath } = useParams();
    const path = rawPath || "";

    const [dirContent, setDirContent] = useState<DirContent | null>(null);

    useEffect(() => {
        api(`/dir/${path}`).then(async resp => {
            setDirContent(await resp.json());
        });
    }, [path]);


    return <>
        {!dirContent && "loading"}
        <div className="flex">
            {dirContent && dirContent.entries.map(e => <DirEntry entry={e} dir_path={path} />)}
        </div>
    </>
}

const DirEntry = (props: { entry: DirEntry, dir_path: string }) => {
    const { entry, dir_path } = props;

    const content = <div className="w-52 h-52 bg-white rounded-3xl m-4 p-4 flex flex-col justify-between items-center" >
        <div>

        {!entry.is_dir && entry.mime && <Icon  {...getFileTypeIconProps({ extension: entry.mime?.split("/")[1], size: 96 })} className="w-48 h-48" />}
        </div>

        <p>{entry.relative_name}</p>
    </div>

    if (entry.is_dir) {
        return <Link id={dir_path + entry.relative_name} to={entry.relative_name + "/"}>{content}</Link>
    } else {
        return <a id={dir_path + entry.relative_name} href={`/api/file/${dir_path + entry.relative_name}`}>{content}</a>;
    }
}