import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { api } from "../api";
import { Icon } from "@fluentui/react";
import { FileIconType, getFileTypeIconProps } from "@fluentui/react-file-type-icons";
import { Disabled } from "../components/Disabled";
import { Header } from "../components/Header";

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
    const [prevPath, setPrevPath] = useState<string | null>(null);

    const navigate = useNavigate();

    useEffect(() => {
        const handlePopState = () => {
            const subpaths = window.location.pathname.split("/");
            subpaths.pop();
            subpaths.pop();
            alert(subpaths.join("/"))
            // console.log("WANT TO GO TO:",subpaths.join("/"))
            navigate(subpaths.join("/"))
        };
        if (window.location.pathname != "/browse/") {
            window.addEventListener("popstate", handlePopState);

            return () => {
                window.removeEventListener("popstate", handlePopState);
            };
        }
    }, [navigate, prevPath]);

    useEffect(() => {
        api(`/dir/${path}`).then(async resp => {
            setDirContent(await resp.json());
            setPrevPath(path);
        });
    }, [path]);

    console.log(getFileTypeIconProps({ extension: "pdf", size: 96 }));

    return <>
        <Header />
        {!dirContent && "loading"}
        <div className="flex flex-wrap">
            {dirContent && dirContent.entries.map(e => <Disabled disabled={prevPath !== path} key={path + e.relative_name}>
                <DirEntry entry={e} dir_path={path} />
            </Disabled>)}
        </div>
    </>
}

const DirEntry = (props: { entry: DirEntry, dir_path: string }) => {
    const { entry, dir_path } = props;

    const content = <div className="w-64 h-24 bg-white rounded-xl m-4 flex flex-row justify-between items-center" >
        {!entry.is_dir && entry.mime && <Icon  {...getFileTypeIconProps({ extension: entry.mime?.split("/")[1], size: 96 })} />}
        {!entry.is_dir && !entry.mime && <Icon {...getFileTypeIconProps({ type: FileIconType.genericFile, size: 96 })} />}
        {entry.is_dir && <Icon {...getFileTypeIconProps({ type: FileIconType.folder, size: 96 })} />}

        <p className="mx-2 text-wrap wrap-break-word max-w-36 line-clamp-3 text-ellipsis">{entry.relative_name}</p>
    </div>

    if (entry.is_dir) {
        return <Link to={entry.relative_name + "/"}>{content}</Link>
    } else {
        return <a href={`/api/file/${dir_path + entry.relative_name}`}>{content}</a>;
    }
}