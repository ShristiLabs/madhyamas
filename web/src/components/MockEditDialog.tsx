import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  useMockCollections,
  useCreateMock,
  useUpdateMock,
  useTestMock,
  useMockRuleAnalytics,
  useMockHitHistory,
  useRollbackMock,
  type MockRule,
  type MockResponse,
  type ResponseConfig,
  type MatchCondition,
  type ConditionalResponse,
  type ProbabilisticResponse,
  type RequestCondition,
  type MockExpiration,
} from "@/lib/api/intercept";
import { useToast } from "@/components/ui/use-toast";
import {
  Plus,
  Trash2,
  Play,
  History,
  BarChart3,
  Info,
  Code,
  Shuffle,
  GitBranch,
  List,
  FileText,
  Clock,
  Tag,
  Folder,
  AlertCircle,
  CheckCircle,
  XCircle,
  RotateCcw,
} from "lucide-react";

interface MockEditDialogProps {
  mock: MockRule | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave?: () => void;
}

const TEMPLATE_VARIABLES = [
  { name: "{{timestamp}}", description: "Unix timestamp in seconds" },
  { name: "{{timestamp_ms}}", description: "Unix timestamp in milliseconds" },
  { name: "{{uuid}}", description: "Random UUID v4" },
  { name: "{{date}}", description: "Current date (YYYY-MM-DD)" },
  { name: "{{time}}", description: "Current time (HH:MM:SS)" },
  { name: "{{datetime}}", description: "ISO 8601 datetime" },
  { name: "{{random_int}}", description: "Random integer 0-999999" },
  { name: "{{random_float}}", description: "Random float 0.00-1.00" },
  { name: "{{request.method}}", description: "HTTP method" },
  { name: "{{request.url}}", description: "Full request URL" },
  { name: "{{request.path}}", description: "URL path" },
  { name: "{{request.host}}", description: "Host/domain" },
  { name: "{{request.query}}", description: "Query string" },
  { name: "{{request.headers.X-Name}}", description: "Request header value" },
  { name: "{{request.query.param}}", description: "Query parameter value" },
  { name: "{{request.body.path}}", description: "JSON body path value" },
];

const DEFAULT_RESPONSE: MockResponse = {
  status_code: 200,
  headers: { "Content-Type": "application/json" },
  body: '{"success": true}',
  template_enabled: false,
};

export function MockEditDialog({
  mock,
  open,
  onOpenChange,
  onSave,
}: MockEditDialogProps) {
  const { toast } = useToast();
  const createMock = useCreateMock();
  const updateMock = useUpdateMock();
  const testMock = useTestMock();
  const { data: collections } = useMockCollections();
  const { data: analytics } = useMockRuleAnalytics(mock?.id || "");
  const { data: hitHistory } = useMockHitHistory(mock?.id || "");
  const rollbackMock = useRollbackMock();

  const isCreating = !mock;

  // Form state
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState("");
  const [collectionId, setCollectionId] = useState<string | undefined>();
  const [matchingConditions, setMatchingConditions] = useState<
    MatchCondition[]
  >([{ type: "url_pattern", pattern: "" }]);
  const [matchingRootOperator, setMatchingRootOperator] = useState<
    "and" | "or"
  >("and");
  const [responseType, setResponseType] =
    useState<ResponseConfig["type"]>("single");
  const [singleResponse, setSingleResponse] =
    useState<MockResponse>(DEFAULT_RESPONSE);
  const [sequenceResponses, setSequenceResponses] = useState<MockResponse[]>([
    DEFAULT_RESPONSE,
  ]);
  const [sequenceCycle, setSequenceCycle] = useState(true);
  const [conditionalResponses, setConditionalResponses] = useState<
    ConditionalResponse[]
  >([]);
  const [defaultResponse, setDefaultResponse] =
    useState<MockResponse>(DEFAULT_RESPONSE);
  const [probabilisticResponses, setProbabilisticResponses] = useState<
    ProbabilisticResponse[]
  >([{ weight: 100, response: DEFAULT_RESPONSE }]);
  const [enabled, setEnabled] = useState(true);
  const [priority, setPriority] = useState(100);
  const [expiration, setExpiration] = useState<MockExpiration | undefined>();
  const [activeTab, setActiveTab] = useState("matching");

  // Test state
  const [testUrl, setTestUrl] = useState("https://api.example.com/test");
  const [testMethod, setTestMethod] = useState("GET");
  const [testResult, setTestResult] = useState<{
    matches: boolean;
    response?: MockResponse;
  } | null>(null);

  // Initialize form when mock changes
  useEffect(() => {
    if (mock) {
      setName(mock.name);
      setDescription(mock.description || "");
      setTags(mock.tags || []);
      setCollectionId(mock.collection_id);
      // Initialize matching conditions from the mock's condition
      setMatchingConditions([mock.condition]);
      setEnabled(mock.enabled);
      setPriority(mock.priority);
      setExpiration(mock.expiration);

      const config = mock.response_config;
      setResponseType(config.type);

      if (config.type === "single" && config.response) {
        setSingleResponse(config.response);
      } else if (config.type === "sequence" && config.responses) {
        setSequenceResponses(config.responses);
        setSequenceCycle(config.loop ?? true);
      } else if (config.type === "conditional") {
        setConditionalResponses(config.conditions || []);
        setDefaultResponse(config.default_response || DEFAULT_RESPONSE);
      } else if (
        config.type === "probabilistic" &&
        config.probabilistic_responses
      ) {
        setProbabilisticResponses(config.probabilistic_responses);
      }
    }
  }, [mock]);

  const buildResponseConfig = (): ResponseConfig => {
    switch (responseType) {
      case "single":
        return { type: "single", response: singleResponse };
      case "sequence":
        return {
          type: "sequence",
          responses: sequenceResponses,
          loop: sequenceCycle,
        };
      case "conditional":
        return {
          type: "conditional",
          conditions: conditionalResponses,
          default_response: defaultResponse,
        };
      case "probabilistic":
        return {
          type: "probabilistic",
          probabilistic_responses: probabilisticResponses,
        };
      default:
        return { type: "single", response: singleResponse };
    }
  };

  // Build the primary condition from matching conditions
  const buildPrimaryCondition = (): MatchCondition => {
    // Use the first condition as the primary one for now
    // In the future, the backend can support composite conditions
    const firstCondition = matchingConditions[0];
    return firstCondition || { type: "url_pattern", pattern: "" };
  };

  const handleSave = async () => {
    // Validate that at least one condition has a value
    const hasValidCondition = matchingConditions.some((c) => {
      if (c.type === "method") return !!c.value;
      if (c.type === "header" || c.type === "query_param") return !!c.name;
      return !!c.pattern;
    });

    if (!name || !hasValidCondition) {
      toast({
        title: "Validation Error",
        description: "Name and at least one matching condition are required",
        variant: "destructive",
      });
      return;
    }

    const primaryCondition = buildPrimaryCondition();

    try {
      if (isCreating) {
        // Create new mock
        await createMock.mutateAsync({
          name,
          description: description || undefined,
          tags,
          collection_id: collectionId,
          condition: primaryCondition,
          response_config: buildResponseConfig(),
          enabled,
          priority,
          expiration,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          version: 1,
          version_history: [],
        });
        toast({ description: "Mock created successfully" });
      } else {
        // Update existing mock
        await updateMock.mutateAsync({
          id: mock!.id,
          rule: {
            ...mock!,
            name,
            description: description || undefined,
            tags,
            collection_id: collectionId,
            condition: primaryCondition,
            response_config: buildResponseConfig(),
            enabled,
            priority,
            expiration,
            updated_at: new Date().toISOString(),
          },
        });
        toast({ description: "Mock updated successfully" });
      }
      onSave?.();
      onOpenChange(false);
    } catch {
      toast({
        title: isCreating ? "Failed to create mock" : "Failed to update mock",
        variant: "destructive",
      });
    }
  };

  const handleTest = async () => {
    if (!mock) return;

    try {
      const result = await testMock.mutateAsync({
        id: mock.id,
        request: {
          url: testUrl,
          method: testMethod,
          headers: {},
        },
      });
      setTestResult(result);
    } catch {
      toast({ title: "Test failed", variant: "destructive" });
    }
  };

  const handleRollback = async (version: number) => {
    if (!mock) return;

    try {
      await rollbackMock.mutateAsync({ id: mock.id, version });
      toast({ description: `Rolled back to version ${version}` });
      onSave?.();
    } catch {
      toast({ title: "Rollback failed", variant: "destructive" });
    }
  };

  const addTag = () => {
    if (tagInput && !tags.includes(tagInput)) {
      setTags([...tags, tagInput]);
      setTagInput("");
    }
  };

  const removeTag = (tag: string) => {
    setTags(tags.filter((t) => t !== tag));
  };

  const addSequenceResponse = () => {
    setSequenceResponses([...sequenceResponses, { ...DEFAULT_RESPONSE }]);
  };

  const removeSequenceResponse = (index: number) => {
    setSequenceResponses(sequenceResponses.filter((_, i) => i !== index));
  };

  const updateSequenceResponse = (index: number, response: MockResponse) => {
    const updated = [...sequenceResponses];
    updated[index] = response;
    setSequenceResponses(updated);
  };

  const addConditionalResponse = () => {
    setConditionalResponses([
      ...conditionalResponses,
      {
        condition: { type: "header_equals", name: "", value: "" },
        response: { ...DEFAULT_RESPONSE },
      },
    ]);
  };

  const removeConditionalResponse = (index: number) => {
    setConditionalResponses(conditionalResponses.filter((_, i) => i !== index));
  };

  const addProbabilisticResponse = () => {
    setProbabilisticResponses([
      ...probabilisticResponses,
      { weight: 10, response: { ...DEFAULT_RESPONSE } },
    ]);
  };

  const removeProbabilisticResponse = (index: number) => {
    setProbabilisticResponses(
      probabilisticResponses.filter((_, i) => i !== index),
    );
  };

  const totalWeight = probabilisticResponses.reduce(
    (sum, r) => sum + r.weight,
    0,
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle>
            {isCreating ? "Create New Mock" : `Edit Mock: ${mock?.name}`}
          </DialogTitle>
          <DialogDescription>
            {isCreating
              ? "Create a new mock with advanced response configuration"
              : "Configure response type, conditions, and advanced settings"}
          </DialogDescription>
        </DialogHeader>

        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="flex-1 overflow-hidden"
        >
          <TabsList
            className={`grid w-full ${isCreating ? "grid-cols-3" : "grid-cols-5"}`}
          >
            <TabsTrigger value="matching">
              <Code className="h-4 w-4 mr-1" />
              Matching
            </TabsTrigger>
            <TabsTrigger value="response">
              <FileText className="h-4 w-4 mr-1" />
              Response
            </TabsTrigger>
            <TabsTrigger value="settings">
              <Tag className="h-4 w-4 mr-1" />
              Settings
            </TabsTrigger>
            {!isCreating && (
              <>
                <TabsTrigger value="test">
                  <Play className="h-4 w-4 mr-1" />
                  Test
                </TabsTrigger>
                <TabsTrigger value="analytics">
                  <BarChart3 className="h-4 w-4 mr-1" />
                  Analytics
                </TabsTrigger>
              </>
            )}
          </TabsList>

          <ScrollArea
            className="flex-1 mt-4 pr-4"
            style={{ height: "calc(90vh - 250px)" }}
          >
            {/* Response Tab */}
            <TabsContent value="response" className="space-y-4 mt-0">
              {/* Response Type Selector */}
              <div className="space-y-2">
                <Label>Response Type</Label>
                <div className="grid grid-cols-4 gap-2">
                  <Button
                    variant={responseType === "single" ? "default" : "outline"}
                    size="sm"
                    onClick={() => setResponseType("single")}
                    className="justify-start"
                  >
                    <FileText className="h-4 w-4 mr-2" />
                    Single
                  </Button>
                  <Button
                    variant={
                      responseType === "sequence" ? "default" : "outline"
                    }
                    size="sm"
                    onClick={() => setResponseType("sequence")}
                    className="justify-start"
                  >
                    <List className="h-4 w-4 mr-2" />
                    Sequence
                  </Button>
                  <Button
                    variant={
                      responseType === "conditional" ? "default" : "outline"
                    }
                    size="sm"
                    onClick={() => setResponseType("conditional")}
                    className="justify-start"
                  >
                    <GitBranch className="h-4 w-4 mr-2" />
                    Conditional
                  </Button>
                  <Button
                    variant={
                      responseType === "probabilistic" ? "default" : "outline"
                    }
                    size="sm"
                    onClick={() => setResponseType("probabilistic")}
                    className="justify-start"
                  >
                    <Shuffle className="h-4 w-4 mr-2" />
                    Probabilistic
                  </Button>
                </div>
              </div>

              {/* Single Response */}
              {responseType === "single" && (
                <ResponseEditor
                  response={singleResponse}
                  onChange={setSingleResponse}
                  showTemplateHelp
                />
              )}

              {/* Sequence Responses */}
              {responseType === "sequence" && (
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Label>Responses ({sequenceResponses.length})</Label>
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger>
                            <Info className="h-4 w-4 text-muted-foreground" />
                          </TooltipTrigger>
                          <TooltipContent>
                            <p>
                              Responses are served in order. Each request gets
                              the next response.
                            </p>
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </div>
                    <div className="flex items-center gap-4">
                      <div className="flex items-center gap-2">
                        <Switch
                          checked={sequenceCycle}
                          onCheckedChange={setSequenceCycle}
                        />
                        <Label className="text-sm">Cycle</Label>
                      </div>
                      <Button size="sm" onClick={addSequenceResponse}>
                        <Plus className="h-4 w-4 mr-1" />
                        Add
                      </Button>
                    </div>
                  </div>
                  <Accordion type="single" collapsible className="space-y-2">
                    {sequenceResponses.map((response, index) => (
                      <AccordionItem
                        key={index}
                        value={`seq-${index}`}
                        className="border rounded-lg"
                      >
                        <AccordionTrigger className="px-4 hover:no-underline">
                          <div className="flex items-center gap-2">
                            <Badge variant="outline">#{index + 1}</Badge>
                            <span className="text-sm">
                              Status {response.status_code}
                            </span>
                            {response.delay_ms && (
                              <Badge variant="secondary" className="text-xs">
                                {response.delay_ms}ms delay
                              </Badge>
                            )}
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-4 pb-4">
                          <ResponseEditor
                            response={response}
                            onChange={(r) => updateSequenceResponse(index, r)}
                          />
                          {sequenceResponses.length > 1 && (
                            <Button
                              variant="destructive"
                              size="sm"
                              className="mt-2"
                              onClick={() => removeSequenceResponse(index)}
                            >
                              <Trash2 className="h-4 w-4 mr-1" />
                              Remove
                            </Button>
                          )}
                        </AccordionContent>
                      </AccordionItem>
                    ))}
                  </Accordion>
                </div>
              )}

              {/* Conditional Responses */}
              {responseType === "conditional" && (
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <Label>Conditions ({conditionalResponses.length})</Label>
                    <Button size="sm" onClick={addConditionalResponse}>
                      <Plus className="h-4 w-4 mr-1" />
                      Add Condition
                    </Button>
                  </div>
                  <Accordion type="single" collapsible className="space-y-2">
                    {conditionalResponses.map((condResp, index) => (
                      <AccordionItem
                        key={index}
                        value={`cond-${index}`}
                        className="border rounded-lg"
                      >
                        <AccordionTrigger className="px-4 hover:no-underline">
                          <div className="flex items-center gap-2">
                            <Badge variant="outline">If</Badge>
                            <span className="text-sm font-mono">
                              {condResp.condition.type}:{" "}
                              {condResp.condition.name ||
                                condResp.condition.path}
                            </span>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-4 pb-4 space-y-4">
                          <ConditionEditor
                            condition={condResp.condition}
                            onChange={(c) => {
                              const updated = [...conditionalResponses];
                              updated[index] = {
                                ...updated[index],
                                condition: c,
                              };
                              setConditionalResponses(updated);
                            }}
                          />
                          <ResponseEditor
                            response={condResp.response}
                            onChange={(r) => {
                              const updated = [...conditionalResponses];
                              updated[index] = {
                                ...updated[index],
                                response: r,
                              };
                              setConditionalResponses(updated);
                            }}
                          />
                          <Button
                            variant="destructive"
                            size="sm"
                            onClick={() => removeConditionalResponse(index)}
                          >
                            <Trash2 className="h-4 w-4 mr-1" />
                            Remove
                          </Button>
                        </AccordionContent>
                      </AccordionItem>
                    ))}
                  </Accordion>

                  <Card>
                    <CardHeader className="py-3">
                      <CardTitle className="text-sm">
                        Default Response
                      </CardTitle>
                      <CardDescription>
                        Used when no conditions match
                      </CardDescription>
                    </CardHeader>
                    <CardContent>
                      <ResponseEditor
                        response={defaultResponse}
                        onChange={setDefaultResponse}
                      />
                    </CardContent>
                  </Card>
                </div>
              )}

              {/* Probabilistic Responses */}
              {responseType === "probabilistic" && (
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Label>Weighted Responses</Label>
                      <Badge variant="secondary">Total: {totalWeight}</Badge>
                    </div>
                    <Button size="sm" onClick={addProbabilisticResponse}>
                      <Plus className="h-4 w-4 mr-1" />
                      Add
                    </Button>
                  </div>
                  <Accordion type="single" collapsible className="space-y-2">
                    {probabilisticResponses.map((probResp, index) => (
                      <AccordionItem
                        key={index}
                        value={`prob-${index}`}
                        className="border rounded-lg"
                      >
                        <AccordionTrigger className="px-4 hover:no-underline">
                          <div className="flex items-center gap-2">
                            <Badge variant="outline">
                              {((probResp.weight / totalWeight) * 100).toFixed(
                                1,
                              )}
                              %
                            </Badge>
                            <span className="text-sm">
                              Status {probResp.response.status_code} (weight:{" "}
                              {probResp.weight})
                            </span>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-4 pb-4 space-y-4">
                          <div className="flex items-center gap-2">
                            <Label className="w-20">Weight</Label>
                            <Input
                              type="number"
                              min={1}
                              value={probResp.weight}
                              onChange={(e) => {
                                const updated = [...probabilisticResponses];
                                updated[index] = {
                                  ...updated[index],
                                  weight: parseInt(e.target.value) || 1,
                                };
                                setProbabilisticResponses(updated);
                              }}
                              className="w-24"
                            />
                          </div>
                          <ResponseEditor
                            response={probResp.response}
                            onChange={(r) => {
                              const updated = [...probabilisticResponses];
                              updated[index] = {
                                ...updated[index],
                                response: r,
                              };
                              setProbabilisticResponses(updated);
                            }}
                          />
                          {probabilisticResponses.length > 1 && (
                            <Button
                              variant="destructive"
                              size="sm"
                              onClick={() => removeProbabilisticResponse(index)}
                            >
                              <Trash2 className="h-4 w-4 mr-1" />
                              Remove
                            </Button>
                          )}
                        </AccordionContent>
                      </AccordionItem>
                    ))}
                  </Accordion>
                </div>
              )}
            </TabsContent>

            {/* Matching Tab */}
            <TabsContent value="matching" className="space-y-4 mt-0">
              <MatchingConditionsEditor
                conditions={matchingConditions}
                onChange={setMatchingConditions}
                rootOperator={matchingRootOperator}
                onRootOperatorChange={setMatchingRootOperator}
              />
            </TabsContent>

            {/* Settings Tab */}
            <TabsContent value="settings" className="space-y-4 mt-0">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Name</Label>
                  <Input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label>Priority</Label>
                  <Input
                    type="number"
                    value={priority}
                    onChange={(e) =>
                      setPriority(parseInt(e.target.value) || 100)
                    }
                  />
                  <p className="text-xs text-muted-foreground">
                    Lower = higher priority
                  </p>
                </div>
              </div>

              <div className="space-y-2">
                <Label>Description</Label>
                <Textarea
                  placeholder="Describe what this mock does..."
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label>Collection</Label>
                <Select
                  value={collectionId || "none"}
                  onValueChange={(v) =>
                    setCollectionId(v === "none" ? undefined : v)
                  }
                >
                  <SelectTrigger>
                    <SelectValue placeholder="No collection" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">No collection</SelectItem>
                    {collections?.map((c) => (
                      <SelectItem key={c.id} value={c.id}>
                        <div className="flex items-center gap-2">
                          <Folder className="h-4 w-4" />
                          {c.name}
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>Tags</Label>
                <div className="flex gap-2">
                  <Input
                    placeholder="Add tag..."
                    value={tagInput}
                    onChange={(e) => setTagInput(e.target.value)}
                    onKeyDown={(e) =>
                      e.key === "Enter" && (e.preventDefault(), addTag())
                    }
                  />
                  <Button onClick={addTag} size="sm">
                    Add
                  </Button>
                </div>
                <div className="flex flex-wrap gap-1 mt-2">
                  {tags.map((tag) => (
                    <Badge
                      key={tag}
                      variant="secondary"
                      className="cursor-pointer"
                      onClick={() => removeTag(tag)}
                    >
                      {tag} ×
                    </Badge>
                  ))}
                </div>
              </div>

              <div className="flex items-center gap-2">
                <Switch checked={enabled} onCheckedChange={setEnabled} />
                <Label>Enabled</Label>
              </div>

              {/* Expiration */}
              <Card>
                <CardHeader className="py-3">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <Clock className="h-4 w-4" />
                    Expiration
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <Select
                    value={expiration?.type || "none"}
                    onValueChange={(v) => {
                      if (v === "none") {
                        setExpiration(undefined);
                      } else if (v === "date_time") {
                        setExpiration({
                          type: "date_time",
                          expires_at: new Date(
                            Date.now() + 86400000,
                          ).toISOString(),
                        });
                      } else if (v === "hit_count") {
                        setExpiration({ type: "hit_count", max_hits: 100 });
                      } else if (v === "duration") {
                        setExpiration({
                          type: "duration",
                          duration_seconds: 3600,
                        });
                      }
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="No expiration" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="none">No expiration</SelectItem>
                      <SelectItem value="date_time">
                        Expire at date/time
                      </SelectItem>
                      <SelectItem value="hit_count">
                        Expire after N hits
                      </SelectItem>
                      <SelectItem value="duration">
                        Expire after duration
                      </SelectItem>
                    </SelectContent>
                  </Select>

                  {expiration?.type === "date_time" && (
                    <Input
                      type="datetime-local"
                      value={expiration.expires_at?.slice(0, 16) || ""}
                      onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                        setExpiration({
                          ...expiration,
                          expires_at: new Date(e.target.value).toISOString(),
                        })
                      }
                    />
                  )}

                  {expiration?.type === "hit_count" && (
                    <div className="flex items-center gap-2">
                      <Label className="w-32">Max hits</Label>
                      <Input
                        type="number"
                        value={expiration.max_hits || 100}
                        onChange={(e) =>
                          setExpiration({
                            ...expiration,
                            max_hits: parseInt(e.target.value) || 100,
                          })
                        }
                      />
                    </div>
                  )}

                  {expiration?.type === "duration" && (
                    <div className="flex items-center gap-2">
                      <Label className="w-32">Duration (sec)</Label>
                      <Input
                        type="number"
                        value={expiration.duration_seconds || 3600}
                        onChange={(e) =>
                          setExpiration({
                            ...expiration,
                            duration_seconds: parseInt(e.target.value) || 3600,
                          })
                        }
                      />
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* Version History */}
              {mock?.version_history && mock.version_history.length > 0 && (
                <Card>
                  <CardHeader className="py-3">
                    <CardTitle className="text-sm flex items-center gap-2">
                      <History className="h-4 w-4" />
                      Version History
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {mock.version_history.map((v) => (
                        <div
                          key={v.version}
                          className="flex items-center justify-between p-2 border rounded"
                        >
                          <div>
                            <span className="font-medium">v{v.version}</span>
                            <span className="text-xs text-muted-foreground ml-2">
                              {new Date(v.timestamp).toLocaleString()}
                            </span>
                            {v.comment && (
                              <p className="text-xs text-muted-foreground">
                                {v.comment}
                              </p>
                            )}
                          </div>
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={() => handleRollback(v.version)}
                          >
                            <RotateCcw className="h-4 w-4 mr-1" />
                            Rollback
                          </Button>
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}
            </TabsContent>

            {/* Test Tab */}
            <TabsContent value="test" className="space-y-4 mt-0">
              <Card>
                <CardHeader className="py-3">
                  <CardTitle className="text-sm">Test Request</CardTitle>
                  <CardDescription>
                    Test if this mock would match a request
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="grid grid-cols-4 gap-2">
                    <Select value={testMethod} onValueChange={setTestMethod}>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="GET">GET</SelectItem>
                        <SelectItem value="POST">POST</SelectItem>
                        <SelectItem value="PUT">PUT</SelectItem>
                        <SelectItem value="DELETE">DELETE</SelectItem>
                        <SelectItem value="PATCH">PATCH</SelectItem>
                      </SelectContent>
                    </Select>
                    <Input
                      className="col-span-3"
                      placeholder="https://api.example.com/test"
                      value={testUrl}
                      onChange={(e) => setTestUrl(e.target.value)}
                    />
                  </div>
                  <Button onClick={handleTest} disabled={testMock.isPending}>
                    <Play className="h-4 w-4 mr-2" />
                    Test Mock
                  </Button>

                  {testResult && (
                    <div
                      className={`p-4 rounded-lg ${testResult.matches ? "bg-green-50 dark:bg-green-950" : "bg-red-50 dark:bg-red-950"}`}
                    >
                      <div className="flex items-center gap-2">
                        {testResult.matches ? (
                          <CheckCircle className="h-5 w-5 text-green-600" />
                        ) : (
                          <XCircle className="h-5 w-5 text-red-600" />
                        )}
                        <span className="font-medium">
                          {testResult.matches
                            ? "Mock would match!"
                            : "Mock would NOT match"}
                        </span>
                      </div>
                      {testResult.matches && testResult.response && (
                        <div className="mt-2 text-sm">
                          <p>Status: {testResult.response.status_code}</p>
                          {testResult.response.body && (
                            <pre className="mt-2 p-2 bg-background rounded text-xs overflow-auto">
                              {testResult.response.body}
                            </pre>
                          )}
                        </div>
                      )}
                    </div>
                  )}
                </CardContent>
              </Card>
            </TabsContent>

            {/* Analytics Tab */}
            <TabsContent value="analytics" className="space-y-4 mt-0">
              {analytics && (
                <div className="grid grid-cols-3 gap-4">
                  <Card>
                    <CardHeader className="py-3">
                      <CardTitle className="text-2xl">
                        {analytics.total_hits}
                      </CardTitle>
                      <CardDescription>Total Hits</CardDescription>
                    </CardHeader>
                  </Card>
                  <Card>
                    <CardHeader className="py-3">
                      <CardTitle className="text-2xl">
                        {analytics.avg_response_time_ms}ms
                      </CardTitle>
                      <CardDescription>Avg Response Time</CardDescription>
                    </CardHeader>
                  </Card>
                  <Card>
                    <CardHeader className="py-3">
                      <CardTitle className="text-2xl">
                        {analytics.min_response_time_ms}-
                        {analytics.max_response_time_ms}ms
                      </CardTitle>
                      <CardDescription>Response Time Range</CardDescription>
                    </CardHeader>
                  </Card>
                </div>
              )}

              {hitHistory && hitHistory.length > 0 && (
                <Card>
                  <CardHeader className="py-3">
                    <CardTitle className="text-sm">Recent Hits</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2 max-h-64 overflow-auto">
                      {hitHistory
                        .slice(-20)
                        .reverse()
                        .map((hit, i) => (
                          <div
                            key={i}
                            className="flex items-center justify-between p-2 border rounded text-sm"
                          >
                            <div className="flex items-center gap-2">
                              <Badge variant="outline">
                                {hit.request_method}
                              </Badge>
                              <span className="font-mono truncate max-w-xs">
                                {hit.request_url}
                              </span>
                            </div>
                            <div className="flex items-center gap-2">
                              <Badge>{hit.response_status}</Badge>
                              <span className="text-muted-foreground">
                                {hit.response_time_ms}ms
                              </span>
                              <span className="text-xs text-muted-foreground">
                                {new Date(hit.timestamp).toLocaleTimeString()}
                              </span>
                            </div>
                          </div>
                        ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {(!hitHistory || hitHistory.length === 0) && (
                <div className="text-center py-8 text-muted-foreground">
                  <AlertCircle className="h-8 w-8 mx-auto mb-2" />
                  <p>No hit history available</p>
                </div>
              )}
            </TabsContent>
          </ScrollArea>
        </Tabs>

        <DialogFooter className="mt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={isCreating ? createMock.isPending : updateMock.isPending}
          >
            {isCreating ? "Create Mock" : "Save Changes"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// Response Editor Component
interface ResponseEditorProps {
  response: MockResponse;
  onChange: (response: MockResponse) => void;
  showTemplateHelp?: boolean;
}

function ResponseEditor({
  response,
  onChange,
  showTemplateHelp,
}: ResponseEditorProps) {
  const [headersText, setHeadersText] = useState(() =>
    Object.entries(response.headers || {})
      .map(([k, v]) => `${k}: ${v}`)
      .join("\n"),
  );

  const parseHeaders = (text: string): Record<string, string> => {
    const headers: Record<string, string> = {};
    text.split("\n").forEach((line) => {
      const [key, ...valueParts] = line.split(":");
      if (key && valueParts.length) {
        headers[key.trim()] = valueParts.join(":").trim();
      }
    });
    return headers;
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-3 gap-4">
        <div className="space-y-2">
          <Label>Status Code</Label>
          <Input
            type="number"
            value={response.status_code}
            onChange={(e) =>
              onChange({
                ...response,
                status_code: parseInt(e.target.value) || 200,
              })
            }
          />
        </div>
        <div className="space-y-2">
          <Label>Delay (ms)</Label>
          <Input
            type="number"
            placeholder="0"
            value={response.delay_ms || ""}
            onChange={(e) =>
              onChange({
                ...response,
                delay_ms: parseInt(e.target.value) || undefined,
              })
            }
          />
        </div>
        <div className="space-y-2">
          <Label>Delay Variance (ms)</Label>
          <Input
            type="number"
            placeholder="0"
            value={response.delay_variance_ms || ""}
            onChange={(e) =>
              onChange({
                ...response,
                delay_variance_ms: parseInt(e.target.value) || undefined,
              })
            }
          />
        </div>
      </div>

      <div className="space-y-2">
        <Label>Headers (one per line: Name: Value)</Label>
        <Textarea
          className="font-mono text-sm"
          placeholder="Content-Type: application/json"
          value={headersText}
          onChange={(e) => {
            setHeadersText(e.target.value);
            onChange({ ...response, headers: parseHeaders(e.target.value) });
          }}
          rows={3}
        />
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label>Response Body</Label>
          <div className="flex items-center gap-2">
            <Switch
              checked={response.template_enabled || false}
              onCheckedChange={(checked) =>
                onChange({ ...response, template_enabled: checked })
              }
            />
            <Label className="text-sm">Enable Templates</Label>
          </div>
        </div>
        <Textarea
          className="font-mono text-sm min-h-[120px]"
          placeholder='{"message": "Hello, World!"}'
          value={response.body || ""}
          onChange={(e) => onChange({ ...response, body: e.target.value })}
        />
      </div>

      {showTemplateHelp && response.template_enabled && (
        <Card className="bg-muted/50">
          <CardHeader className="py-2">
            <CardTitle className="text-xs">
              Available Template Variables
            </CardTitle>
          </CardHeader>
          <CardContent className="py-2">
            <div className="grid grid-cols-2 gap-1 text-xs">
              {TEMPLATE_VARIABLES.map((v) => (
                <div key={v.name} className="flex items-center gap-2">
                  <code className="bg-background px-1 rounded">{v.name}</code>
                  <span className="text-muted-foreground truncate">
                    {v.description}
                  </span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

// Condition Editor Component
interface ConditionEditorProps {
  condition: RequestCondition;
  onChange: (condition: RequestCondition) => void;
}

function ConditionEditor({ condition, onChange }: ConditionEditorProps) {
  return (
    <div className="space-y-4 p-4 border rounded-lg bg-muted/30">
      <div className="space-y-2">
        <Label>Condition Type</Label>
        <Select
          value={condition.type}
          onValueChange={(v) =>
            onChange({ ...condition, type: v as RequestCondition["type"] })
          }
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="header_equals">Header Equals</SelectItem>
            <SelectItem value="header_regex">Header Matches (Regex)</SelectItem>
            <SelectItem value="query_param">Query Parameter</SelectItem>
            <SelectItem value="body_contains">Body Contains</SelectItem>
            <SelectItem value="body_json_path">JSON Path</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {(condition.type === "header_equals" ||
        condition.type === "header_regex") && (
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label>Header Name</Label>
            <Input
              placeholder="X-Custom-Header"
              value={condition.name || ""}
              onChange={(e) => onChange({ ...condition, name: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <Label>
              {condition.type === "header_regex" ? "Pattern" : "Value"}
            </Label>
            <Input
              placeholder={
                condition.type === "header_regex"
                  ? ".*pattern.*"
                  : "expected-value"
              }
              value={
                condition.type === "header_regex"
                  ? condition.pattern || ""
                  : condition.value || ""
              }
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                onChange({
                  ...condition,
                  [condition.type === "header_regex" ? "pattern" : "value"]:
                    e.target.value,
                })
              }
            />
          </div>
        </div>
      )}

      {condition.type === "query_param" && (
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label>Parameter Name</Label>
            <Input
              placeholder="userId"
              value={condition.name || ""}
              onChange={(e) => onChange({ ...condition, name: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <Label>Value</Label>
            <Input
              placeholder="123"
              value={condition.value || ""}
              onChange={(e) =>
                onChange({ ...condition, value: e.target.value })
              }
            />
          </div>
        </div>
      )}

      {condition.type === "body_contains" && (
        <div className="space-y-2">
          <Label>Pattern (Regex)</Label>
          <Input
            placeholder=".*error.*"
            value={condition.pattern || ""}
            onChange={(e) =>
              onChange({ ...condition, pattern: e.target.value })
            }
          />
        </div>
      )}

      {condition.type === "body_json_path" && (
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label>JSON Path</Label>
            <Input
              placeholder="$.user.id"
              value={condition.path || ""}
              onChange={(e) => onChange({ ...condition, path: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <Label>Expected Value</Label>
            <Input
              placeholder="123"
              value={condition.expected || ""}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                onChange({ ...condition, expected: e.target.value })
              }
            />
          </div>
        </div>
      )}
    </div>
  );
}

// Matching Conditions Editor Component - supports multiple conditions with AND/OR
interface MatchingConditionsEditorProps {
  conditions: MatchCondition[];
  onChange: (conditions: MatchCondition[]) => void;
  rootOperator: "and" | "or";
  onRootOperatorChange: (operator: "and" | "or") => void;
}

const MATCH_TYPE_OPTIONS = [
  { value: "url_pattern", label: "URL Pattern (Regex)" },
  { value: "host", label: "Host/Domain" },
  { value: "path", label: "Path Pattern" },
  { value: "method", label: "HTTP Method" },
  { value: "header", label: "Request Header" },
  { value: "query_param", label: "Query Parameter" },
];

const HTTP_METHODS = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
];

function MatchingConditionsEditor({
  conditions,
  onChange,
  rootOperator,
  onRootOperatorChange,
}: MatchingConditionsEditorProps) {
  const addCondition = () => {
    onChange([...conditions, { type: "url_pattern", pattern: "" }]);
  };

  const removeCondition = (index: number) => {
    if (conditions.length > 1) {
      onChange(conditions.filter((_, i) => i !== index));
    }
  };

  const updateCondition = (index: number, updates: Partial<MatchCondition>) => {
    const updated = [...conditions];
    updated[index] = { ...updated[index], ...updates };
    onChange(updated);
  };

  return (
    <div className="space-y-4">
      {/* Header with operator toggle */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Label className="text-base font-medium">Match Conditions</Label>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger>
                <Info className="h-4 w-4 text-muted-foreground" />
              </TooltipTrigger>
              <TooltipContent className="max-w-xs">
                <p>
                  Add multiple conditions to match requests. Use AND to require
                  all conditions, or OR to match any condition.
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <Button size="sm" variant="outline" onClick={addCondition}>
          <Plus className="h-4 w-4 mr-1" />
          Add Condition
        </Button>
      </div>

      {/* Root operator toggle - only show if more than one condition */}
      {conditions.length > 1 && (
        <div className="flex items-center gap-2 p-3 bg-muted/50 rounded-lg">
          <span className="text-sm text-muted-foreground">Match</span>
          <div className="flex rounded-md border overflow-hidden">
            <button
              type="button"
              className={`px-3 py-1 text-sm font-medium transition-colors ${
                rootOperator === "and"
                  ? "bg-primary text-primary-foreground"
                  : "bg-background hover:bg-muted"
              }`}
              onClick={() => onRootOperatorChange("and")}
            >
              ALL
            </button>
            <button
              type="button"
              className={`px-3 py-1 text-sm font-medium transition-colors ${
                rootOperator === "or"
                  ? "bg-primary text-primary-foreground"
                  : "bg-background hover:bg-muted"
              }`}
              onClick={() => onRootOperatorChange("or")}
            >
              ANY
            </button>
          </div>
          <span className="text-sm text-muted-foreground">
            of the following conditions
          </span>
        </div>
      )}

      {/* Conditions list */}
      <div className="space-y-3">
        {conditions.map((cond, index) => (
          <div
            key={index}
            className="p-4 border rounded-lg bg-background space-y-3"
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1 grid grid-cols-2 gap-3">
                {/* Match Type */}
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">
                    Match Type
                  </Label>
                  <Select
                    value={cond.type}
                    onValueChange={(v) =>
                      updateCondition(index, {
                        type: v as MatchCondition["type"],
                        pattern: "",
                        name: "",
                        value: "",
                      })
                    }
                  >
                    <SelectTrigger className="h-9">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {MATCH_TYPE_OPTIONS.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {/* Value input based on type */}
                {cond.type === "method" ? (
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">
                      HTTP Method
                    </Label>
                    <Select
                      value={cond.value || "GET"}
                      onValueChange={(v) =>
                        updateCondition(index, { value: v })
                      }
                    >
                      <SelectTrigger className="h-9">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {HTTP_METHODS.map((method) => (
                          <SelectItem key={method} value={method}>
                            {method}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                ) : cond.type === "header" || cond.type === "query_param" ? (
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">
                      {cond.type === "header"
                        ? "Header Name"
                        : "Parameter Name"}
                    </Label>
                    <Input
                      className="h-9"
                      placeholder={
                        cond.type === "header" ? "Content-Type" : "userId"
                      }
                      value={cond.name || ""}
                      onChange={(e) =>
                        updateCondition(index, { name: e.target.value })
                      }
                    />
                  </div>
                ) : (
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">
                      Pattern
                    </Label>
                    <Input
                      className="h-9 font-mono"
                      placeholder={
                        cond.type === "url_pattern"
                          ? ".*api/users.*"
                          : cond.type === "host"
                            ? "api.example.com"
                            : "/api/v1/.*"
                      }
                      value={cond.pattern || ""}
                      onChange={(e) =>
                        updateCondition(index, { pattern: e.target.value })
                      }
                    />
                  </div>
                )}
              </div>

              {/* Remove button */}
              {conditions.length > 1 && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-9 w-9 p-0 text-muted-foreground hover:text-destructive"
                  onClick={() => removeCondition(index)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              )}
            </div>

            {/* Additional value field for header/query_param */}
            {(cond.type === "header" || cond.type === "query_param") && (
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">
                  Value Pattern (Regex)
                </Label>
                <Input
                  className="font-mono"
                  placeholder=".*expected-value.*"
                  value={cond.pattern || ""}
                  onChange={(e) =>
                    updateCondition(index, { pattern: e.target.value })
                  }
                />
              </div>
            )}

            {/* Operator indicator between conditions */}
            {index < conditions.length - 1 && (
              <div className="flex items-center justify-center pt-2">
                <Badge
                  variant="secondary"
                  className="text-xs font-medium uppercase"
                >
                  {rootOperator}
                </Badge>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Help text */}
      <p className="text-xs text-muted-foreground">
        Use regex patterns for flexible matching. Example:{" "}
        <code className="bg-muted px-1 rounded">.*api/v[12]/users.*</code>
      </p>
    </div>
  );
}
